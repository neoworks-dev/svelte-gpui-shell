import type { Context, Plugin } from '@neoworks/extension-system'
import { requireService } from '../../lib/services.js'
import { registerFunction, RetainedTopics } from '../../lib/bus.js'
import type { BusService } from '../../lib/bus.js'
import { listApps } from '../apps/backend.js'
import type { App } from '../apps/backend.js'
import {
  directoryOf,
  fileExists,
  homeDirectory,
  joinPath,
  makeDirectory,
  moveToTrash,
  readDirectoryEntries,
  renameTo,
  watchDirectory,
} from '../../system/fs.js'
import { launchDetached, output } from '../../system/proc.js'
import { applicationDirectories, desktopDirectory } from '../../system/xdg.js'

// files: the desktop folder as a retained bus topic, plus the operations the
// desktop's icons and context menu need. The desktop view draws the icons;
// this side only knows the filesystem.
//
//   files.desktop    {path, entries: [{name, path, directory, icon, image}]}
//   files:open       {path} → {ok} | {error}
//   files:openwith   {path, command} → {ok} | {error}
//   files:handlers   {path} → {mime, handlers: [{id, name, command}]}
//   files:newfolder  {dir?, name?} → {path} | {error}
//   files:rename     {path, name} → {path} | {error}
//   files:trash      {paths} → {trashed, errors}
//   files:images     {dir?} → {path, entries: [{name, path}]}
//
// `gio` does three parts: the freedesktop trash layout (gio trash), the content
// type behind "Open With" (gio info, gio mime), and opening with the default
// handler (gio open).
//
// icon is a freedesktop icon name the view hands straight to the icon theme;
// image marks entries the view shows as their own preview.

interface FilesConfig {
  desktopDir?: string
  wallpaperDir?: string
}

const NEW_FOLDER_NAME = 'New Folder'
const WATCH_DEBOUNCE_MS = 120

export interface DesktopEntry {
  name: string
  path: string
  directory: boolean
  icon: string
  image: boolean
}

const filesExtension: Plugin.Object<FilesConfig | undefined> = {
  name: 'files',
  inject: ['bus'],
  apply(context, config) {
    const bus = requireService<BusService>(context, 'bus')
    const topics = new RetainedTopics(bus)
    const desktopDir = desktopDirOf(config)
    const wallpaperDir = wallpaperDirOf(config)
    context.effect(() => () => topics.withdrawAll())
    watchDesktop(context, topics, desktopDir)
    registerFunction(context, bus, 'files:open', (data) => openEntry(data))
    registerFunction(context, bus, 'files:openwith', (data) => openWith(data))
    registerFunction(context, bus, 'files:handlers', (data) => listHandlers(data))
    registerFunction(context, bus, 'files:newfolder', async (data) => newFolder(await desktopDir, data))
    registerFunction(context, bus, 'files:rename', (data) => renameEntry(data))
    registerFunction(context, bus, 'files:trash', (data) => trashEntries(data))
    registerFunction(context, bus, 'files:images', async (data) => listImages(await wallpaperDir, data))
  },
}

export default filesExtension

function desktopDirOf(config: FilesConfig | undefined): Promise<string> {
  if (config !== undefined && config.desktopDir !== undefined && config.desktopDir !== '') {
    return Promise.resolve(config.desktopDir)
  }
  return desktopDirectory()
}

async function wallpaperDirOf(config: FilesConfig | undefined): Promise<string> {
  if (config !== undefined && config.wallpaperDir !== undefined && config.wallpaperDir !== '') {
    return config.wallpaperDir
  }
  return joinPath(await homeDirectory(), 'Pictures', 'Wallpapers')
}

function watchDesktop(context: Context, topics: RetainedTopics, dirPromise: Promise<string>): void {
  context.effect(() => {
    let stop = () => {}
    let disposed = false
    void dirPromise.then(async (dir) => {
      await refreshDesktop(topics, dir)
      if (!disposed) {
        stop = watchDebounced(dir, () => void refreshDesktop(topics, dir))
      }
    })
    return () => {
      disposed = true
      stop()
    }
  })
}

async function refreshDesktop(topics: RetainedTopics, dir: string): Promise<void> {
  topics.set('files.desktop', { path: dir, entries: await readEntries(dir) })
}

// A directory that does not exist is the common case (no ~/Desktop at all):
// the topic stays an empty list and the view shows nothing, rather than the
// extension failing to mount.
function watchDebounced(dir: string, refresh: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null
  const stop = watchDirectory(dir, () => {
    if (timer !== null) {
      clearTimeout(timer)
    }
    timer = setTimeout(refresh, WATCH_DEBOUNCE_MS)
  })
  return () => {
    if (timer !== null) {
      clearTimeout(timer)
    }
    stop()
  }
}

async function readEntries(dir: string): Promise<DesktopEntry[]> {
  const listing = await readDirectoryEntries(dir)
  const entries = listing
    .filter((entry) => !entry.name.startsWith('.'))
    .map((entry) => entryOf(dir, entry.name, entry.directory))
  entries.sort(compareEntries)
  return entries
}

function entryOf(dir: string, name: string, directory: boolean): DesktopEntry {
  return {
    name,
    path: joinPath(dir, name),
    directory,
    icon: iconNameOf(name, directory),
    image: isImage(name),
  }
}

// Folders first, then names case-insensitively — the ordering every file
// manager uses, and the one an icon grid reads as sorted.
export function compareEntries(left: DesktopEntry, right: DesktopEntry): number {
  if (left.directory !== right.directory) {
    if (left.directory) {
      return -1
    }
    return 1
  }
  return left.name.toLowerCase().localeCompare(right.name.toLowerCase())
}

const IMAGE_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.webp', '.avif', '.bmp'])

const ICON_BY_EXTENSION: Record<string, string> = {
  '.pdf': 'application-pdf',
  '.zip': 'application-x-archive',
  '.tar': 'application-x-archive',
  '.gz': 'application-x-archive',
  '.xz': 'application-x-archive',
  '.7z': 'application-x-archive',
  '.mp3': 'audio-x-generic',
  '.flac': 'audio-x-generic',
  '.wav': 'audio-x-generic',
  '.opus': 'audio-x-generic',
  '.mp4': 'video-x-generic',
  '.mkv': 'video-x-generic',
  '.webm': 'video-x-generic',
  '.sh': 'text-x-script',
  '.desktop': 'application-x-executable',
}

export function iconNameOf(name: string, directory: boolean): string {
  if (directory) {
    return 'folder'
  }
  const extension = extensionOf(name)
  if (IMAGE_EXTENSIONS.has(extension)) {
    return 'image-x-generic'
  }
  const icon = ICON_BY_EXTENSION[extension]
  if (icon === undefined) {
    return 'text-x-generic'
  }
  return icon
}

export function isImage(name: string): boolean {
  return IMAGE_EXTENSIONS.has(extensionOf(name))
}

// The containers GStreamer's playbin opens in practice; the wallpaper picker
// lists these next to the images.
const VIDEO_EXTENSIONS = new Set(['.mp4', '.m4v', '.webm', '.mkv', '.mov', '.avi'])

export function isVideo(name: string): boolean {
  return VIDEO_EXTENSIONS.has(extensionOf(name))
}

// openEntry hands the path to the desktop's own handler. The opened
// application outlives the shell because it starts in its own session.
async function openEntry(data: unknown): Promise<unknown> {
  const request = data as { path?: string }
  if (typeof request.path !== 'string' || request.path === '') {
    return { error: 'path is required' }
  }
  return replyOf(await launchDetached(['gio', 'open', request.path]))
}

// openWith runs one application's Exec line against the path. The command
// comes from files:handlers, which has already stripped the .desktop field
// codes, so the path is appended as a single quoted argument.
async function openWith(data: unknown): Promise<unknown> {
  const request = data as { path?: string; command?: string }
  if (typeof request.path !== 'string' || request.path === '') {
    return { error: 'path is required' }
  }
  if (typeof request.command !== 'string' || request.command.trim() === '') {
    return { error: 'command is required' }
  }
  const line = `${request.command} ${shellQuote(request.path)}`
  return replyOf(await launchDetached(['sh', '-c', line]))
}

function replyOf(error: string): unknown {
  if (error !== '') {
    return { error }
  }
  return { ok: true }
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'\\''`)}'`
}

// listHandlers answers "Open With": the installed applications that claim the
// path's content type. With no claimants the whole list is returned: a menu of
// everything still beats a menu of nothing.
async function listHandlers(data: unknown): Promise<unknown> {
  const request = data as { path?: string }
  if (typeof request.path !== 'string' || request.path === '') {
    return { error: 'path is required' }
  }
  const mime = await contentTypeOf(request.path)
  const installed = await listApps(await applicationDirectories())
  const claimed = await claimantIds(mime)
  const matching = installed.filter((app) => claimed.has(`${app.id}.desktop`))
  if (matching.length === 0) {
    return { mime, handlers: handlersOf(installed) }
  }
  return { mime, handlers: handlersOf(matching) }
}

function handlersOf(apps: App[]): Array<{ id: string; name: string; command: string }> {
  return apps
    .filter((app) => app.exec !== '')
    .map((app) => ({ id: app.id, name: app.name, command: app.exec }))
}

// `gio mime TYPE` lists the desktop ids under "Registered applications:" and
// "Recommended applications:".
async function claimantIds(mime: string): Promise<Set<string>> {
  const ids = new Set<string>()
  if (mime === '') {
    return ids
  }
  for (const line of (await output(['gio', 'mime', mime])).split('\n')) {
    if (line.startsWith('\t') && line.trim().endsWith('.desktop')) {
      ids.add(line.trim())
    }
  }
  return ids
}

async function contentTypeOf(path: string): Promise<string> {
  const text = await output(['gio', 'info', '-a', 'standard::content-type', path])
  const match = text.match(/standard::content-type: (\S+)/)
  if (match === null) {
    return ''
  }
  return match[1]
}

// newFolder picks the first free "New Folder", "New Folder 2", … so repeated
// invocations never collide and never overwrite.
async function newFolder(desktopDir: string, data: unknown): Promise<unknown> {
  const request = data as { dir?: string; name?: string }
  const dir = stringOr(request.dir, desktopDir)
  const name = stringOr(request.name, NEW_FOLDER_NAME)
  if (!isSafeName(name)) {
    return { error: 'name must not contain a path separator' }
  }
  const path = await freePath(dir, name)
  await makeDirectory(path)
  if (!(await fileExists(path))) {
    return { error: `cannot create ${path}` }
  }
  return { path }
}

async function renameEntry(data: unknown): Promise<unknown> {
  const request = data as { path?: string; name?: string }
  if (typeof request.path !== 'string' || request.path === '') {
    return { error: 'path is required' }
  }
  if (typeof request.name !== 'string' || !isSafeName(request.name)) {
    return { error: 'name must be a plain file name' }
  }
  const target = joinPath(directoryOf(request.path), request.name)
  if (await fileExists(target)) {
    return { error: `${request.name} already exists` }
  }
  const error = await renameTo(request.path, target)
  if (error !== '') {
    return { error }
  }
  return { path: target }
}

async function trashEntries(data: unknown): Promise<unknown> {
  const request = data as { paths?: unknown }
  const paths = stringsOf(request.paths)
  if (paths.length === 0) {
    return { error: 'paths is required' }
  }
  const trashed: string[] = []
  const errors: string[] = []
  for (const path of paths) {
    await collectTrashResult(path, trashed, errors)
  }
  return { trashed, errors }
}

async function collectTrashResult(path: string, trashed: string[], errors: string[]): Promise<void> {
  const error = await moveToTrash(path)
  if (error === '') {
    trashed.push(path)
    return
  }
  errors.push(`${path}: ${error}`)
}

// listImages backs the wallpaper picker. Videos are listed too: a live
// wallpaper is chosen the same way a still one is.
async function listImages(wallpaperDir: string, data: unknown): Promise<unknown> {
  const request = data as { dir?: string }
  const dir = stringOr(request.dir, wallpaperDir)
  const listing = await readDirectoryEntries(dir)
  const entries = listing
    .filter((entry) => !entry.directory && !entry.name.startsWith('.'))
    .filter((entry) => isImage(entry.name) || isVideo(entry.name))
    .map((entry) => ({ name: entry.name, path: joinPath(dir, entry.name) }))
  entries.sort((left, right) => left.name.localeCompare(right.name))
  return { path: dir, entries }
}

// freePath appends " 2", " 3", … before the extension until nothing is in the
// way, the numbering a file manager uses for both copies and trashed names.
export async function freePath(dir: string, name: string): Promise<string> {
  const direct = joinPath(dir, name)
  if (!(await fileExists(direct))) {
    return direct
  }
  const extension = extensionOf(name)
  const stem = name.slice(0, name.length - extension.length)
  for (let index = 2; ; index += 1) {
    const candidate = joinPath(dir, `${stem} ${index}${extension}`)
    if (!(await fileExists(candidate))) {
      return candidate
    }
  }
}

export function isSafeName(name: string): boolean {
  if (name === '' || name === '.' || name === '..') {
    return false
  }
  return !name.includes('/')
}

function extensionOf(name: string): string {
  const dot = name.lastIndexOf('.')
  if (dot <= 0) {
    return ''
  }
  return name.slice(dot).toLowerCase()
}

function stringOr(value: unknown, fallback: string): string {
  if (typeof value !== 'string' || value === '') {
    return fallback
  }
  return value
}

function stringsOf(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return []
  }
  return value.filter((entry): entry is string => typeof entry === 'string' && entry !== '')
}
