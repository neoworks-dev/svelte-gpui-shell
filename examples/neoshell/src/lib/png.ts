// A PNG writer for the window previews: the captured frame arrives as raw
// RGBA and the image primitive reads files. The pixels go in as stored
// (uncompressed) deflate blocks, which costs size and nothing else.

const SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
const MAX_STORED_BLOCK = 0xffff
const ADLER_MODULUS = 65521
const BYTES_PER_PIXEL = 4

let crcTable: Uint32Array | null = null

function table(): Uint32Array {
  if (crcTable !== null) {
    return crcTable
  }
  const built = new Uint32Array(256)
  for (let index = 0; index < 256; index += 1) {
    let value = index
    for (let bit = 0; bit < 8; bit += 1) {
      if ((value & 1) !== 0) {
        value = 0xedb88320 ^ (value >>> 1)
      } else {
        value = value >>> 1
      }
    }
    built[index] = value >>> 0
  }
  crcTable = built
  return built
}

export function crc32(bytes: Uint8Array): number {
  const lookup = table()
  let crc = 0xffffffff
  for (const byte of bytes) {
    crc = lookup[(crc ^ byte) & 0xff] ^ (crc >>> 8)
  }
  return (crc ^ 0xffffffff) >>> 0
}

export function adler32(bytes: Uint8Array): number {
  let low = 1
  let high = 0
  for (const byte of bytes) {
    low = (low + byte) % ADLER_MODULUS
    high = (high + low) % ADLER_MODULUS
  }
  return ((high << 16) | low) >>> 0
}

function writeUint32(target: Uint8Array, offset: number, value: number): void {
  target[offset] = (value >>> 24) & 0xff
  target[offset + 1] = (value >>> 16) & 0xff
  target[offset + 2] = (value >>> 8) & 0xff
  target[offset + 3] = value & 0xff
}

function chunk(type: string, data: Uint8Array): Uint8Array {
  const result = new Uint8Array(12 + data.length)
  writeUint32(result, 0, data.length)
  for (let index = 0; index < 4; index += 1) {
    result[4 + index] = type.charCodeAt(index)
  }
  result.set(data, 8)
  writeUint32(result, 8 + data.length, crc32(result.subarray(4, 8 + data.length)))
  return result
}

// Rows of RGBA, each prefixed with filter type 0 (none).
function scanlines(width: number, height: number, stride: number, rgba: Uint8Array): Uint8Array {
  const rowBytes = width * BYTES_PER_PIXEL
  const result = new Uint8Array(height * (rowBytes + 1))
  for (let row = 0; row < height; row += 1) {
    const target = row * (rowBytes + 1)
    result[target] = 0
    result.set(rgba.subarray(row * stride, row * stride + rowBytes), target + 1)
  }
  return result
}

function zlibStored(data: Uint8Array): Uint8Array {
  const blocks = Math.max(1, Math.ceil(data.length / MAX_STORED_BLOCK))
  const result = new Uint8Array(2 + data.length + blocks * 5 + 4)
  result[0] = 0x78
  result[1] = 0x01
  let target = 2
  for (let block = 0; block < blocks; block += 1) {
    const start = block * MAX_STORED_BLOCK
    const length = Math.min(MAX_STORED_BLOCK, data.length - start)
    const isLast = block === blocks - 1
    result[target] = 0
    if (isLast) {
      result[target] = 1
    }
    result[target + 1] = length & 0xff
    result[target + 2] = length >>> 8
    result[target + 3] = ~length & 0xff
    result[target + 4] = (~length >>> 8) & 0xff
    result.set(data.subarray(start, start + length), target + 5)
    target += 5 + length
  }
  writeUint32(result, target, adler32(data))
  return result
}

export function encodePng(width: number, height: number, stride: number, rgba: Uint8Array): Uint8Array {
  const header = new Uint8Array(13)
  writeUint32(header, 0, width)
  writeUint32(header, 4, height)
  header[8] = 8
  header[9] = 6
  const pieces = [
    Uint8Array.from(SIGNATURE),
    chunk('IHDR', header),
    chunk('IDAT', zlibStored(scanlines(width, height, stride, rgba))),
    chunk('IEND', new Uint8Array(0)),
  ]
  const result = new Uint8Array(pieces.reduce((total, piece) => total + piece.length, 0))
  let offset = 0
  for (const piece of pieces) {
    result.set(piece, offset)
    offset += piece.length
  }
  return result
}
