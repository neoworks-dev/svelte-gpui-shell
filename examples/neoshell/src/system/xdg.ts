import { environment, homeDirectory, readTextFile, systemPaths } from './fs.js'
import { joinPath } from './paths.js'
import { userDirectoryOf } from './userDirs.js'

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
