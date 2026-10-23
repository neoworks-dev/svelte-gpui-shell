// How a passwd entry and a display name read, as the lock screen shows them.

export function realNameOf(passwdLine: string, userName: string): string {
  const fields = passwdLine.trim().split(':')
  if (fields.length < 5) {
    return userName
  }
  const name = fields[4].split(',')[0].trim()
  if (name === '') {
    return userName
  }
  return name
}

export function initialsOf(name: string): string {
  return name
    .split(/\s+/)
    .filter((part) => part !== '')
    .slice(0, 2)
    .map((part) => part[0].toUpperCase())
    .join('')
}
