// What a file's extension says it is, for the desktop's icons and the
// wallpaper picker.

export const IMAGE_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.webp', '.avif', '.bmp'])

// The containers GStreamer's playbin opens in practice; the wallpaper picker
// lists these next to the images.
export const VIDEO_EXTENSIONS = new Set(['.mp4', '.m4v', '.webm', '.mkv', '.mov', '.avi'])

export function extensionOf(name: string): string {
  const dot = name.lastIndexOf('.')
  if (dot <= 0) {
    return ''
  }
  return name.slice(dot).toLowerCase()
}

export function isImage(name: string): boolean {
  return IMAGE_EXTENSIONS.has(extensionOf(name))
}

export function isVideo(name: string): boolean {
  return VIDEO_EXTENSIONS.has(extensionOf(name))
}
