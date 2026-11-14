import { GIFEncoder } from 'gifenc'

import { LoopEncoder, maskUnchanged } from '../../lib/readme/encode'

import { readGif } from '../gif'

const HEIGHT = 4
const WIDTH  = 6

// Fills a frame with `ground`, then paints the pixels from `from` to `to` in `ink`.
function frame(ground: number[], ink: number[], from = 0, to = 0): Uint8Array {
  const rgba = new Uint8Array(WIDTH * HEIGHT * 4)
  for (let at = 0; at < WIDTH * HEIGHT; at++) rgba.set([...at >= from && at < to ? ink : ground, 255], at * 4)
  return rgba
}

const GROUND = [255, 255, 255]
const INK    = [138, 128, 203]
const FRAMES = [frame(GROUND, INK), frame(GROUND, INK, 2, 9), frame(GROUND, INK, 2, 9), frame(GROUND, INK, 5, 7)]

// Encodes `frames` against a palette drawn from all of them.
function encode(frames: readonly Uint8Array[]): Uint8Array {
  const encoder = new LoopEncoder(20, HEIGHT, WIDTH, frames)
  frames.forEach(rgba => encoder.add(rgba))
  return encoder.bytes()
}

describe('LoopEncoder', () => {
  it('writes the same bytes for the same frames', () => {
    expect(encode(FRAMES)).toStrictEqual(encode(FRAMES))
  })

  it('loops forever and holds every frame for 20 ms', () => {
    const gif = encode(FRAMES)
    expect(Buffer.from(gif.subarray(0, 6)).toString('latin1')).toBe('GIF89a')
    expect(readGif(gif).loops).toBe(0)
    expect(readGif(gif).frames.map(({ delay }) => delay)).toStrictEqual([2, 2, 2, 2])
  })

  it('keeps each later frame over the one before it through one transparent index', () => {
    const [first, ...later] = readGif(encode(FRAMES)).frames
    expect(first!.fields & 1).toBe(0)
    later.forEach(({ fields, transparent }) => {
      expect(fields).toBe(1 << 2 | 1)
      expect(transparent).toBe(LoopEncoder.palette(FRAMES).length)
    })
  })

  it('draws a palette holding each color the samples carry', () => {
    expect(LoopEncoder.palette(FRAMES)).toStrictEqual(expect.arrayContaining([GROUND, INK]))
  })

  it('marks only the pixels the frame before it shares as transparent', () => {
    const index = Uint8Array.of(0, 1, 2, 3)
    maskUnchanged(index, 255, Uint8Array.of(0, 9, 2, 9))
    expect(index).toStrictEqual(Uint8Array.of(255, 1, 255, 3))
  })

  it('quantizes to at most 255 colors, leaving the last index for transparency', () => {
    const ramp = Uint8Array.from({ length: 4096 * 4 }, (_, at) => at % 4 === 3 ? 255 : (at >> 2 >> 4 * (at % 4) & 15) << 4)
    expect(LoopEncoder.palette([ramp]).length).toBeLessThanOrEqual(255)
  })

  it('writes the header the gifenc encoder writes', () => {
    const gif = GIFEncoder()
    gif.writeFrame(new Uint8Array(WIDTH * HEIGHT), WIDTH, HEIGHT, { palette: [GROUND] })
    expect(encode(FRAMES).subarray(0, 10)).toStrictEqual(gif.bytesView().subarray(0, 10))
  })
})

describe('readGif', () => {
  it('reports no delay for a frame no graphic control block precedes', () => {
    const image = [0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 0]
    const gif   = Uint8Array.of(
      ...Buffer.from('GIF89a', 'latin1'), 1, 0, 1, 0, 0, 0, 0,
      0x21, 0xf9, 4, 1, 2, 0, 0, 0, ...image,
      ...image,
      0x3b
    )
    expect(readGif(gif).frames.map(({ delay }) => delay)).toStrictEqual([2, 0])
  })
})
