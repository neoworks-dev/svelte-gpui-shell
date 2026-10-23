import { files, processes } from '@neoworks/svelte-gpui'
import type { FilePaths } from '@neoworks/svelte-gpui'

// The filesystem calls the extension backends make. Every one is a promise,
// because the host does the work on a thread. Failures keep the shape the
// callers already branch on: null, false, an empty list, or an error string.

let pathsPromise: Promise<FilePaths> | null = null

export function systemPaths(): Promise<FilePaths> {
  if (pathsPromise === null) {
    pathsPromise = files.paths()
  }
  return pathsPromise
}

export async function homeDirectory(): Promise<string> {
  return (await systemPaths()).home
}

export async function runtimeDirectory(): Promise<string> {
  return (await systemPaths()).runtime
}

export async function environment(): Promise<Record<string, string>> {
  return processes.env()
}

export async function readTextFile(path: string): Promise<string | null> {
  try {
    return await files.readText(path)
  } catch {
    return null
  }
}

export async function writeTextFile(path: string, contents: string): Promise<boolean> {
  try {
    await files.writeText(path, contents)
    return true
  } catch (error) {
    console.error(`fs: cannot write ${path}:`, error)
    return false
  }
}

export async function fileExists(path: string): Promise<boolean> {
  try {
    return await files.exists(path)
  } catch {
    return false
  }
}

export async function isDirectory(path: string): Promise<boolean> {
  try {
    return (await files.stat(path)).kind === 'directory'
  } catch {
    return false
  }
}

export async function makeDirectory(path: string): Promise<void> {
  try {
    await files.mkdir(path, { recursive: true })
  } catch (error) {
    console.error(`fs: cannot create ${path}:`, error)
  }
}

export interface DirectoryEntry {
  name: string
  directory: boolean
}

export async function readDirectory(path: string): Promise<string[]> {
  const entries = await readDirectoryEntries(path)
  return entries.map((entry) => entry.name)
}

export async function readDirectoryEntries(path: string): Promise<DirectoryEntry[]> {
  try {
    const listing = await files.list(path)
    return listing.map((entry) => ({ name: entry.name, directory: entry.kind === 'directory' }))
  } catch {
    return []
  }
}

export async function renameTo(path: string, target: string): Promise<string> {
  try {
    await files.rename(path, target)
    return ''
  } catch (error) {
    return String(error)
  }
}

// gio implements the freedesktop trash layout, the .trashinfo file and the
// collision numbering, so none of it is restated here.
export async function moveToTrash(path: string): Promise<string> {
  try {
    const result = await processes.run(['gio', 'trash', '--', path])
    if (result.status === 0) {
      return ''
    }
    return result.stderr.trim()
  } catch (error) {
    return String(error)
  }
}

// Milliseconds since the epoch; 0 when the path is gone.
export async function modifiedAt(path: string): Promise<number> {
  try {
    return (await files.stat(path)).modified
  } catch {
    return 0
  }
}

// watchDirectory reports the base name of every file that changed inside the
// directory. Watching the directory rather than the file is what lets a config
// file be followed before it exists, and survives the truncate-and-rewrite an
// editor performs.
export function watchDirectory(path: string, onChange: (name: string) => void): () => void {
  return files.watch(path, (event) => onChange(baseNameOf(event.path)), {
    onerror: (error) => console.error(`fs: cannot watch ${path}:`, error.message),
  })
}

export function joinPath(...parts: string[]): string {
  return parts.join('/').replace(/\/{2,}/g, '/')
}

export function baseNameOf(path: string): string {
  const cut = path.lastIndexOf('/')
  if (cut < 0) {
    return path
  }
  return path.slice(cut + 1)
}

export function directoryOf(path: string): string {
  const cut = path.lastIndexOf('/')
  if (cut < 0) {
    return '.'
  }
  if (cut === 0) {
    return '/'
  }
  return path.slice(0, cut)
}
