// The windowcapture helper's output: three little-endian uint32 (width,
// height, stride in bytes), then `height` rows of `stride` bytes, R8G8B8A8.

const HEADER_BYTES = 12

export interface CaptureFrame {
  width: number
  height: number
  stride: number
  pixels: Uint8Array
}

export function parseCaptureFrame(raw: Uint8Array): CaptureFrame {
  if (raw.length < HEADER_BYTES) {
    throw new Error('windowcapture wrote no frame')
  }
  const header = new DataView(raw.buffer, raw.byteOffset, HEADER_BYTES)
  const width = header.getUint32(0, true)
  const height = header.getUint32(4, true)
  const stride = header.getUint32(8, true)
  if (raw.length !== HEADER_BYTES + height * stride) {
    throw new Error('windowcapture wrote a truncated frame')
  }
  return { width, height, stride, pixels: raw.subarray(HEADER_BYTES) }
}
