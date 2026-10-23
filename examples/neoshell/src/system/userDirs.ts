// Reads one entry of ~/.config/user-dirs.dirs.

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

