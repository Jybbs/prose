import fs   from 'node:fs'
import path from 'node:path'

import { SCHEMES }   from '../../lib/shared/palette'
import { readmeDir } from '../../lib/shared/paths'
import { readGif }   from '../gif'

const LIMIT  = 6_000_000
const SHAPE  = { frames: 540, height: 972, width: 1216 }

describe.each(SCHEMES)('the %s Sandbox recording', scheme => {
  const bytes = fs.readFileSync(path.join(readmeDir(import.meta.url), `sandbox-${scheme}.gif`))
  const gif   = readGif(bytes)

  it('holds the frames and the size of the recording', () => {
    expect({ frames: gif.frames.length, height: gif.height, width: gif.width }).toStrictEqual(SHAPE)
  })

  it('loops forever and holds every frame for 20 ms', () => {
    expect(gif.loops).toBe(0)
    expect(new Set(gif.frames.map(({ delay }) => delay))).toStrictEqual(new Set([2]))
  })

  it('draws every frame from one shared palette', () => {
    expect(gif.palette).toBeGreaterThan(0)
    expect(gif.frames.filter(({ local }) => local)).toStrictEqual([])
  })

  it(`stays under ${LIMIT} bytes`, () => {
    expect(bytes.length).toBeLessThan(LIMIT)
  })
})
