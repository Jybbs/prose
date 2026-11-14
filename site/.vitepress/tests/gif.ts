const APPLICATION = 0xff
const CONTROL     = 0xf9
const EXTENSION   = 0x21
const IMAGE       = 0x2c
const TRAILER     = 0x3b

interface Gif {
  frames  : Frame[]
  height  : number
  loops   : number | null
  palette : number
  width   : number
}

// Holds a frame's graphic control fields, its delay in hundredths of a
// second, its transparent index, and whether it carries its own palette.
interface Frame {
  delay       : number
  fields      : number
  local       : boolean
  transparent : number
}

// Walks a GIF's blocks without decoding any pixels, reading its logical
// screen size, its global palette's entry count, the loop count of its
// NETSCAPE2.0 extension, and each frame's graphic control fields.
export function readGif(bytes: Uint8Array): Gif {
  const view  = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const table = (packed: number): number => packed & 0x80 ? 3 * 2 ** ((packed & 7) + 1) : 0
  const gif   : Gif = {
    frames  : [],
    height  : view.getUint16(8, true),
    loops   : null,
    palette : table(bytes[10]!) / 3,
    width   : view.getUint16(6, true)
  }
  let at      = 13 + table(bytes[10]!)
  let control = { delay: 0, fields: 0, transparent: 0 }
  const skip  = (): void => {
    while (bytes[at]! > 0) at += bytes[at]! + 1
    at++
  }
  while (bytes[at] !== TRAILER) {
    const block = bytes[at]
    if (block === EXTENSION) {
      const label = bytes[at + 1]
      at += 2
      if (label === CONTROL) {
        control = { delay: view.getUint16(at + 2, true), fields: bytes[at + 1]!, transparent: bytes[at + 4]! }
      }
      if (label === APPLICATION && Buffer.from(bytes.subarray(at + 1, at + 12)).toString('latin1') === 'NETSCAPE2.0') {
        gif.loops = view.getUint16(at + 14, true)
      }
      skip()
    }
    else if (block === IMAGE) {
      const packed = bytes[at + 9]!
      gif.frames.push({ ...control, local: (packed & 0x80) !== 0 })
      at += 11 + table(packed)
      skip()
    }
    else throw new Error(`unknown GIF block ${block} at byte ${at}`)
  }
  return gif
}
