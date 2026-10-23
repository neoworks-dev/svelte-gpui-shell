import { files, processes } from '@neoworks/svelte-gpui'
import { parseCaptureFrame } from '../../lib/captureFrame.js'
import { encodePng } from '../../lib/png.js'
import { joinPath, makeDirectory, systemPaths } from '../../system/fs.js'

// captureWindow grabs one frame of a window through the windowcapture helper
// (tools/windowcapture, speaking hyprland-toplevel-export-v1) and answers with
// a PNG file in the cache, fitting inside `width` × `height`. The helper
// writes raw RGBA to stdout, which a process call returns as bytes.
//
// The process allow list is fixed when the shell starts, so the helper is run
// by name and has to be on PATH as neoshell-windowcapture.

const HELPER = 'neoshell-windowcapture'
const DEFAULT_SIZE = 240
const CAPTURE_TIMEOUT_MS = 5000

let frameCounter = 0
const lastImages = new Map<string, string>()
// The view swaps to the new frame within a frame or two; the old file goes after.
const REMOVE_DELAY_MS = 2000

export async function captureWindow(data: unknown): Promise<unknown> {
  const args = data as { address?: string; width?: number; height?: number }
  if (typeof args.address !== 'string' || !/^(0x)?[0-9a-f]+$/i.test(args.address)) {
    return { error: 'address is required' }
  }
  try {
    return await capture(args.address, sizeOf(args.width), sizeOf(args.height))
  } catch (error) {
    return { error: String(error) }
  }
}

async function capture(address: string, width: number, height: number): Promise<unknown> {
  const directory = joinPath((await systemPaths()).cache, 'neoshell')
  await makeDirectory(directory)
  const result = await processes.run([HELPER, address, String(width), String(height)], {
    encoding: 'bytes',
    timeout: CAPTURE_TIMEOUT_MS,
  })
  if (result.status !== 0) {
    throw new Error(result.stderr.trim())
  }
  const frame = parseCaptureFrame(result.stdout)
  frameCounter += 1
  const image = joinPath(directory, `capture-${address}-${frameCounter}.png`)
  await files.writeBytes(image, encodePng(frame.width, frame.height, frame.stride, frame.pixels))
  removeLater(lastImages.get(address))
  lastImages.set(address, image)
  return { image, width: frame.width, height: frame.height }
}

function removeLater(path: string | undefined): void {
  if (path === undefined) {
    return
  }
  setTimeout(() => {
    files.remove(path).catch(() => {})
  }, REMOVE_DELAY_MS)
}

function sizeOf(value: number | undefined): number {
  if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
    return Math.round(value)
  }
  return DEFAULT_SIZE
}
