import { fc, test } from '@fast-check/vitest'

import { heroGrid, watermarkHeight } from '../../lib/landing/hero-tiling'

const tiling = { rowStridePx: 100, stampPerColPx: 200, terminusGapPx: 40 }

describe('heroGrid', () => {
  it.each([
    { name: 'floors columns and rows at three', height: 0,    layerWidth: 0,    cols: 3, rows: 3  },
    { name: 'scales columns by stamp width',    height: 0,    layerWidth: 1200, cols: 6, rows: 3  },
    { name: 'scales rows by the row stride',    height: 1000, layerWidth: 0,    cols: 3, rows: 11 }
  ])('$name', ({ height, layerWidth, cols, rows }) => {
    expect(heroGrid(height, layerWidth, tiling)).toEqual({ cols, rows })
  })

  test.prop([fc.integer({ min: -500, max: 4000 }), fc.integer({ min: 0, max: 4000 })])(
    'never thins below the three-by-three floor',
    (height, layerWidth) => {
      const { cols, rows } = heroGrid(height, layerWidth, tiling)
      expect(cols).toBeGreaterThanOrEqual(3)
      expect(rows).toBeGreaterThanOrEqual(3)
    }
  )

  it('falls back to the production tiling when no options are given', () => {
    expect(heroGrid(0, 0)).toEqual({ cols: 3, rows: 3 })
  })
})

describe('watermarkHeight', () => {
  it.each([
    { name: 'is zero when the carousel is absent',      heroTop: 100, terminusTop: null, height: 0   },
    { name: 'spans from the hero down less the gap',    heroTop: 100, terminusTop: 500,  height: 360 },
    { name: 'goes negative when the carousel overlaps', heroTop: 0,   terminusTop: 30,   height: -10 }
  ])('$name', ({ heroTop, terminusTop, height }) => {
    expect(watermarkHeight(heroTop, terminusTop, tiling)).toBe(height)
  })

  it('uses the production gap by default', () => {
    expect(watermarkHeight(100, 500)).toBe(360)
  })
})
