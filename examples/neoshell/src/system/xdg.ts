import { environment, homeDirectory, joinPath, readTextFile, systemPaths } from './fs.js'

// The freedesktop directories the backends read: where .desktop files live
// and where the user keeps their desktop folder.

// applicationDirectories mirrors the XDG order, user first so a user override
// wins over a system entry of the same id.
export async function applicationDirectories(): Promise<string[]> {
  const paths = await systemPaths()
  const variables = await environment()
  const dirs = [joinPath(paths.data, 'applications')]
  for (const base of systemDataDirectories(variables)) {
    dirs.push(joinPath(base, 'applications'))
  }
  return dirs
}

function systemDataDirectories(variables: Record<string, string>): string[] {
  const declared = variables.XDG_DATA_DIRS
  if (declared === undefined || declared === '') {
    return ['/usr/local/share', '/usr/share']
  }
  return declared.split(':').filter((entry) => entry !== '')
}

// desktopDirectory reads XDG_DESKTOP_DIR from user-dirs.dirs, falling back to
// ~/Desktop as GLib does.
export async function desktopDirectory(): Promise<string> {
  const paths = await systemPaths()
  const text = await readTextFile(joinPath(paths.config, 'user-dirs.dirs'))
  const declared = userDirectoryOf(text, 'XDG_DESKTOP_DIR', paths.home)
  if (declared !== null) {
    return declared
  }
  return joinPath(await homeDirectory(), 'Desktop')
}

export function userDirectoryOf(text: string | null, key: string, home: string): string | null {
  if (text === null) {
    return null
  }
  for (const line of text.split('\n')) {
    const value = valueOfLine(line, key)
    if (value !== null) {
      return value.replace('$HOME', home)
    }
  }
  return null
}

function valueOfLine(line: string, key: string): string | null {
  const match = line.trim().match(new RegExp(`^${key}="(.*)"$`))
  if (match === null) {
    return null
  }
  return match[1]
}

