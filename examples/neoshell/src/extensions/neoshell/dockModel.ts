import { recordOf } from '../../lib/record.js'

// What the dock shows and how it matches windows to launchers, kept apart from
// the view so each rule reads, and tests, on its own.

export interface DockApp {
  id: string
  name: string
  exec: string
  icon: string
  wmClass: string
}

export interface OpenWindow {
  address: string
  wmClass: string
  title: string
  workspace: string
  pid: number
  // The process name behind the window, from /proc: what a terminal with a
  // custom class still has in common with its desktop entry.
  command: string
}

const GENERIC_ICON = 'application-x-executable'
const STEAM_ICON = 'steam'

export function dockAppOf(raw: unknown): DockApp {
  const record = recordOf(raw)
  const app: DockApp = { id: '', name: '', exec: '', icon: '', wmClass: '' }
  if (typeof record.id === 'string') {
    app.id = record.id
  }
  if (typeof record.name === 'string') {
    app.name = record.name
  }
  if (app.name === '' && typeof record.label === 'string') {
    app.name = record.label
  }
  if (typeof record.exec === 'string') {
    app.exec = record.exec
  }
  if (typeof record.icon === 'string') {
    app.icon = record.icon
  }
  if (typeof record.wmClass === 'string') {
    app.wmClass = record.wmClass
  }
  return app
}

// dockEntries keeps the pinned order, then appends the open windows' apps so
// the dock always shows what is actually running.
export function dockEntries(available: DockApp[], selected: DockApp[], open: OpenWindow[]): DockApp[] {
  const shown = [...selected]
  const taken = new Set(shown.map((app) => app.id.toLowerCase()))
  for (const window of open) {
    const app = entryForWindow(available, window)
    if (!taken.has(app.id.toLowerCase())) {
      taken.add(app.id.toLowerCase())
      shown.push(app)
    }
  }
  return shown
}

// resolvePinned prefers dock.pinned (names resolved against the catalog) and
// falls back to dock.apps, whose records already carry exec and icon.
export function resolvePinned(available: DockApp[], wanted: string[], configured: DockApp[]): DockApp[] {
  if (wanted.length === 0) {
    return configured
  }
  const selected: DockApp[] = []
  for (const entry of wanted) {
    const app = findApp(available, entry)
    if (app !== null) {
      selected.push(app)
    }
  }
  return selected
}

function findApp(available: DockApp[], wanted: string): DockApp | null {
  const needle = wanted.toLowerCase()
  for (const app of available) {
    if (app.id.toLowerCase() === needle || app.name.toLowerCase() === needle) {
      return app
    }
  }
  return null
}

// entryForWindow resolves a window against the desktop-entry catalog. The
// class is tried against StartupWMClass, the entry id and the name; failing
// that the window's process name is tried against each entry's command, so a
// terminal started with its own class keeps a tile of its own but borrows the
// terminal's icon. What matches nothing gets a generic icon, or Steam's for a
// Steam game.
export function entryForWindow(available: DockApp[], window: OpenWindow): DockApp {
  const byClass = findByClass(available, window.wmClass)
  if (byClass !== null) {
    return byClass
  }
  const fallback: DockApp = {
    id: window.wmClass,
    name: window.wmClass,
    exec: '',
    icon: iconForUnknown(window.wmClass),
    wmClass: window.wmClass,
  }
  const byCommand = findByCommand(available, window.command)
  if (byCommand !== null) {
    fallback.exec = byCommand.exec
    fallback.icon = byCommand.icon
  }
  return fallback
}

function findByClass(available: DockApp[], wmClass: string): DockApp | null {
  const needle = wmClass.toLowerCase()
  for (const app of available) {
    if (matchesClass(app, needle)) {
      return app
    }
  }
  return null
}

function matchesClass(app: DockApp, needle: string): boolean {
  return (
    app.wmClass.toLowerCase() === needle ||
    app.id.toLowerCase() === needle ||
    app.name.toLowerCase() === needle
  )
}

// The kernel truncates a process name to 15 characters, so the command's
// basename is compared by prefix.
function findByCommand(available: DockApp[], command: string): DockApp | null {
  if (command === '') {
    return null
  }
  for (const app of available) {
    if (commandBasename(app.exec).startsWith(command)) {
      return app
    }
  }
  return null
}

function commandBasename(exec: string): string {
  const first = exec.trim().split(/\s+/)[0]
  return first.slice(first.lastIndexOf('/') + 1)
}

function iconForUnknown(wmClass: string): string {
  if (wmClass.startsWith('steam_app_')) {
    return STEAM_ICON
  }
  return GENERIC_ICON
}

export function openWindowsOf(data: unknown): OpenWindow[] {
  if (!Array.isArray(data)) {
    return []
  }
  const windows: OpenWindow[] = []
  for (const raw of data) {
    const window = openWindowOf(recordOf(raw))
    if (window !== null) {
      windows.push(window)
    }
  }
  return windows
}

function openWindowOf(record: Record<string, unknown>): OpenWindow | null {
  if (typeof record.class !== 'string' || record.class === '' || typeof record.address !== 'string') {
    return null
  }
  const window: OpenWindow = {
    address: record.address,
    wmClass: record.class,
    title: '',
    workspace: '',
    pid: 0,
    command: '',
  }
  if (typeof record.title === 'string') {
    window.title = record.title
  }
  if (typeof record.pid === 'number' && record.pid > 0) {
    window.pid = record.pid
  }
  const workspace = recordOf(record.workspace)
  if (typeof workspace.name === 'string') {
    window.workspace = workspace.name
  }
  return window
}

export function openClassesOf(windows: OpenWindow[]): string[] {
  const classes = new Set(windows.map((window) => window.wmClass))
  return [...classes].sort((left, right) => left.localeCompare(right))
}

export function pinnedNamesOf(snapshot: unknown): string[] {
  const dockConfig = recordOf(recordOf(snapshot).dock)
  if (!Array.isArray(dockConfig.pinned)) {
    return []
  }
  return dockConfig.pinned.filter((value) => typeof value === 'string')
}

export function configuredAppsOf(snapshot: unknown): DockApp[] {
  const dockConfig = recordOf(recordOf(snapshot).dock)
  if (!Array.isArray(dockConfig.apps)) {
    return []
  }
  return dockConfig.apps.map(dockAppOf).filter((app) => app.id !== '')
}

export function focusClass(app: DockApp): string {
  if (app.wmClass !== '') {
    return app.wmClass
  }
  return app.id
}

export function windowsOfApp(openWindows: OpenWindow[], app: DockApp): OpenWindow[] {
  const needle = focusClass(app).toLowerCase()
  return openWindows.filter((window) => window.wmClass.toLowerCase() === needle)
}

export function workspaceLabel(window: OpenWindow): string {
  if (window.workspace === '') {
    return ''
  }
  return `Workspace ${window.workspace}`
}

// Magnification: the hovered icon grows and its neighbours grow less and slide
// outwards, so the row spreads around the cursor.
const MAGNIFY = [
  { scale: 1.45, shift: 0 },
  { scale: 1.2, shift: 6.4 },
  { scale: 1.06, shift: 8 },
]

export interface Magnification {
  scale: number
  shift: number
}

export function magnificationOf(index: number, hoveredIndex: number): Magnification {
  const distance = Math.abs(index - hoveredIndex)
  if (hoveredIndex < 0 || distance >= MAGNIFY.length) {
    return { scale: 1, shift: 0 }
  }
  const step = MAGNIFY[distance]
  return { scale: step.scale, shift: Math.sign(index - hoveredIndex) * step.shift }
}
