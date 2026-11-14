declare module '@fontsource/*'
declare module '@fontsource-variable/*'

declare module 'gifenc' {
  type Palette = number[][]
  interface FrameOptions {
    delay?            : number
    dispose?          : number
    palette?          : Palette
    repeat?           : number
    transparent?      : boolean
    transparentIndex? : number
  }
  interface Encoder {
    bytesView  : () => Uint8Array
    finish     : () => void
    writeFrame : (index: Uint8Array, width: number, height: number, options?: FrameOptions) => void
  }
  export function GIFEncoder(): Encoder
  export function applyPalette(rgba: Uint8Array | Uint8ClampedArray, palette: Palette): Uint8Array
  export function quantize(rgba: Uint8Array | Uint8ClampedArray, maxColors: number): Palette
}
