import { applyPalette, GIFEncoder, quantize } from 'gifenc'

const COLORS      = 255
const KEEP        = 1
const TRANSPARENT = [0, 0, 0]

// Encodes RGBA frames of one size into an endlessly looping GIF against one
// palette, where every frame after the first marks each pixel whose palette
// index matches the frame before it as transparent, leaving the earlier pixel
// in place.
export class LoopEncoder {
  readonly #gif     = GIFEncoder()
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

  add(rgba: Uint8Array): void {
    const index    = applyPalette(rgba, this.#palette)
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
    const clear = this.#palette.length
    index.forEach((value, at) => { if (value === previous[at]) index[at] = clear })
    this.#gif.writeFrame(index, this.width, this.height, {
      delay            : this.delay,
      dispose          : KEEP,
      transparent      : true,
      transparentIndex : clear
    })
  }

  bytes(): Uint8Array {
    this.#gif.finish()
    return this.#gif.bytesView()
  }
}
