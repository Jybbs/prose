import { GIFEncoder, nearestColorIndex, quantize } from 'gifenc'

const COLORS      = 255
const KEEP        = 1
const TRANSPARENT = [0, 0, 0]

// Encodes RGBA frames of one size into an endlessly looping GIF against one
// palette, where every frame after the first marks each pixel whose palette
// index matches the frame before it as transparent, leaving the earlier pixel
// in place.
export class LoopEncoder {
  readonly #gif     = GIFEncoder()
  readonly #nearest = new Map<number, number>()
  readonly #palette : number[][]
  #previous         : Uint8Array | null = null

  constructor(
    readonly delay  : number,
    readonly height : number,
    readonly width  : number,
    samples         : readonly Uint8Array[]
  ) {
    this.#palette = LoopEncoder.palette(samples)
  }

  // Quantizes the samples together into one palette of at most `COLORS`
  // entries, which leaves the index after them free for transparency. The
  // samples are copied into a sheet that owns its whole buffer, since
  // `quantize` reads the buffer from offset zero.
  static palette(samples: readonly Uint8Array[]): number[][] {
    const sheet = new Uint8Array(samples.reduce((size, sample) => size + sample.length, 0))
    let offset  = 0
    for (const sample of samples) {
      sheet.set(sample, offset)
      offset += sample.length
    }
    return quantize(sheet, COLORS)
  }

  // Maps each pixel to its nearest palette entry, caching the entry by the
  // pixel's exact color. gifenc's `applyPalette` caches one entry per rgb565
  // bin, filled by whichever pixel of the bin a frame reaches first, so one
  // color could take a different entry from one frame to the next.
  index(rgba: Uint8Array): Uint8Array {
    const pixels = new Uint32Array(rgba.buffer, rgba.byteOffset, rgba.length / 4)
    return Uint8Array.from(pixels, pixel => {
      const color = pixel & 0xffffff
      return this.#nearest.get(color) ?? this.#nearestTo(color)
    })
  }

  #nearestTo(color: number): number {
    const at = nearestColorIndex(this.#palette, [color & 0xff, color >> 8 & 0xff, color >> 16 & 0xff])
    this.#nearest.set(color, at)
    return at
  }

  add(rgba: Uint8Array): void {
    const index    = this.index(rgba)
    const previous = this.#previous
    this.#previous = index.slice()
    if (previous === null) {
      this.#gif.writeFrame(index, this.width, this.height, {
        delay   : this.delay,
        palette : [...this.#palette, TRANSPARENT],
        repeat  : 0
      })
      return
    }
    maskUnchanged(index, this.#palette.length, previous)
    this.#gif.writeFrame(index, this.width, this.height, {
      delay            : this.delay,
      dispose          : KEEP,
      transparent      : true,
      transparentIndex : this.#palette.length
    })
  }

  bytes(): Uint8Array {
    this.#gif.finish()
    return this.#gif.bytesView()
  }
}

// Replaces each entry of `index` matching `previous` at the same pixel with
// `clear`, the palette's transparent index.
export function maskUnchanged(index: Uint8Array, clear: number, previous: Uint8Array): void {
  index.forEach((value, at) => { if (value === previous[at]) index[at] = clear })
}
