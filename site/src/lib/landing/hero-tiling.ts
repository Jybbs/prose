import * as fc from 'fast-check'

import { ROW_STRIDE_PX } from './hero-stamps'

export interface HeroTilingOptions {
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

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const tiling = { ...HERO_TILING, rowStridePx: 100, stampPerColPx: 200 }

  describe('heroGrid', () => {
    test.each([
      { name: 'floors columns and rows at three', height: 0,    layerWidth: 0,    cols: 3, rows: 3 },
      { name: 'scales columns by stamp width',    height: 0,    layerWidth: 1200, cols: 6, rows: 3 },
      { name: 'scales rows by the row stride',    height: 1000, layerWidth: 0,    cols: 3, rows: 11 }
    ])('$name', ({ height, layerWidth, cols, rows }) => {
      expect(heroGrid(height, layerWidth, tiling)).toEqual({ cols, rows })
    })

    test('never thins below the three-by-three floor', () => {
      fc.assert(fc.property(fc.integer({ min: -500, max: 4000 }), fc.integer({ min: 0, max: 4000 }), (height, layerWidth) => {
        const { cols, rows } = heroGrid(height, layerWidth, tiling)
        expect(cols).toBeGreaterThanOrEqual(3)
        expect(rows).toBeGreaterThanOrEqual(3)
      }))
    })

    test('falls back to the production tiling when no options are given', () => {
      expect(heroGrid(0, 0)).toEqual({ cols: 3, rows: 3 })
    })
  })

  describe('watermarkHeight', () => {
    test.each([
      { name: 'is zero when the carousel is absent', heroTop: 100, terminusTop: null, height: 0 },
      { name: 'spans from the hero down less the gap', heroTop: 100, terminusTop: 500, height: 360 },
      { name: 'can go negative when the carousel overlaps', heroTop: 0, terminusTop: 30, height: -10 }
    ])('$name', ({ heroTop, terminusTop, height }) => {
      expect(watermarkHeight(heroTop, terminusTop, tiling)).toBe(height)
    })

    test('uses the production gap by default', () => {
      expect(watermarkHeight(100, 500)).toBe(500 - 100 - HERO_TILING.terminusGapPx)
    })
  })
}
