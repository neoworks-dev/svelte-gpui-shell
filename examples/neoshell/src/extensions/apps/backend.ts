import type { Plugin } from '@neoworks/extension-system'
import { requireService } from '../../lib/services.js'
import { registerFunction } from '../../lib/bus.js'
import type { BusService } from '../../lib/bus.js'
import { joinPath, readDirectory, readTextFile } from '../../system/fs.js'
import { launchDetached } from '../../system/proc.js'
import { applicationDirectories } from '../../system/xdg.js'

// apps: freedesktop .desktop entries and detached launching. Two functions,
// deliberately primitive — no name-resolution wrapper, callers match against
// the returned list themselves:
//
//   apps:list    → App[] (visible applications, deduped by id, sorted by name)
//   apps:launch  {command} → {ok} | {error}   detached, in its own session

export interface App {
  id: string
  name: string
  exec: string
  icon: string
  wmClass: string
  categories: string[]
  mimeTypes: string[]
}

interface AppsConfig {
  searchDirs?: string[]
}

const appsExtension: Plugin.Object<AppsConfig | undefined> = {
  name: 'apps',
  inject: ['bus'],
  apply(context, config) {
    const bus = requireService<BusService>(context, 'bus')
    registerFunction(context, bus, 'apps:list', async () => listApps(await resolveSearchDirs(config)))
    registerFunction(context, bus, 'apps:launch', (data) => launchApp(data))
  },
}

export default appsExtension

function resolveSearchDirs(config: AppsConfig | undefined): Promise<string[]> {
  if (config !== undefined && config.searchDirs !== undefined) {
    return Promise.resolve(config.searchDirs)
  }
  return applicationDirectories()
}

export async function listApps(searchDirs: string[]): Promise<App[]> {
  const seen = new Set<string>()
  const result: App[] = []
  for (const dir of searchDirs) {
    await collectApps(dir, seen, result)
  }
  result.sort((left, right) => left.name.toLowerCase().localeCompare(right.name.toLowerCase()))
  return result
}

async function collectApps(dir: string, seen: Set<string>, result: App[]): Promise<void> {
  for (const fileName of await desktopFilesIn(dir)) {
    const id = fileName.slice(0, -'.desktop'.length)
    if (seen.has(id)) {
      continue
    }
    const app = await parseDesktopFile(joinPath(dir, fileName), id)
    if (app === null) {
      continue
    }
    seen.add(id)
    result.push(app)
  }
}

async function desktopFilesIn(dir: string): Promise<string[]> {
  const entries = await readDirectory(dir)
  return entries.filter((entry) => entry.endsWith('.desktop'))
}

// parseDesktopFile reads the [Desktop Entry] group; null for entries that
// should not be shown or launched (non-applications, NoDisplay, Hidden).
export async function parseDesktopFile(path: string, id: string): Promise<App | null> {
  const raw = await readTextFile(path)
  if (raw === null) {
    return null
  }
  const app: App = {
    id,
    name: '',
    exec: '',
    icon: '',
    wmClass: '',
    categories: [],
    mimeTypes: [],
  }
  let inEntry = false
  for (const rawLine of raw.split('\n')) {
    const line = rawLine.trim()
    if (line.startsWith('[')) {
      inEntry = line === '[Desktop Entry]'
      continue
    }
    if (!inEntry) {
      continue
    }
    if (!applyDesktopLine(app, line)) {
      return null
    }
  }
  return app
}

// applyDesktopLine folds one key=value line into app; false means the whole
// entry is not a launchable application.
function applyDesktopLine(app: App, line: string): boolean {
  const separatorIndex = line.indexOf('=')
  if (separatorIndex < 0) {
    return true
  }
  const key = line.slice(0, separatorIndex)
  const value = line.slice(separatorIndex + 1)
  if (key === 'Type' && value !== 'Application') {
    return false
  }
  if ((key === 'NoDisplay' || key === 'Hidden') && value.toLowerCase() === 'true') {
    return false
  }
  if (key === 'Name' && app.name === '') {
    app.name = value
  }
  if (key === 'Exec') {
    app.exec = cleanExec(value)
  }
  if (key === 'Icon') {
    app.icon = value
  }
  if (key === 'StartupWMClass') {
    app.wmClass = value
  }
  if (key === 'Categories') {
    app.categories = value.split(';').filter((category) => category !== '')
  }
  if (key === 'MimeType') {
    app.mimeTypes = value.split(';').filter((mimeType) => mimeType !== '')
  }
  return true
}

// cleanExec strips .desktop field codes (%U, %f, …) the shell cannot expand.
function cleanExec(exec: string): string {
  return exec.replace(/%[fFuUdDnNickvm]/g, '').replace(/\s+/g, ' ').trim()
}

// launchApp starts a command detached, in its own session, so the compositor
// maps the window and the app outlives the shell. sh -c keeps .desktop Exec
// lines with env prefixes working.
async function launchApp(data: unknown): Promise<unknown> {
  const args = data as { command?: string }
  if (typeof args.command !== 'string' || args.command.trim() === '') {
    return { error: 'command is required' }
  }
  const error = await launchDetached(['sh', '-c', args.command])
  if (error !== '') {
    return { error }
  }
  return { ok: true }
}
