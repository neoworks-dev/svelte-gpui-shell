// Path strings, with no I/O behind them.

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
