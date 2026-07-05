import { ROW_STRIDE_PX } from './hero-stamps'

interface HeroTilingOptions {
  rowStridePx   : number
  stampPerColPx : number
  terminusGapPx : number
}

const HERO_TILING: HeroTilingOptions = {
  rowStridePx   : ROW_STRIDE_PX,
  stampPerColPx : 240,
  terminusGapPx : 40
}

export function heroGrid(
  height     : number,
  layerWidth : number,
  options    : HeroTilingOptions = HERO_TILING
): { cols: number, rows: number } {
  return {
    cols : Math.max(3, Math.round(layerWidth / options.stampPerColPx)),
    rows : Math.max(3, Math.ceil(Math.max(height, 0) / options.rowStridePx) + 1)
  }
}

export function watermarkHeight(
  heroTop     : number,
  terminusTop : number | null,
  options     : HeroTilingOptions = HERO_TILING
): number {
  return terminusTop === null ? 0 : terminusTop - heroTop - options.terminusGapPx
}
