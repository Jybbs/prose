import * as rulerScale from '../../lib/sandbox/ruler-scale'

describe('steppedLength', () => {
  it.each([
    { expected: 87,         key: 'ArrowDown',  shiftKey: false },
    { expected: 87,         key: 'ArrowLeft',  shiftKey: false },
    { expected: 89,         key: 'ArrowRight', shiftKey: false },
    { expected: 89,         key: 'ArrowUp',    shiftKey: false },
    { expected: 78,         key: 'ArrowLeft',  shiftKey: true  },
    { expected: 98,         key: 'ArrowRight', shiftKey: true  },
    { expected: 78,         key: 'PageDown',   shiftKey: false },
    { expected: 98,         key: 'PageUp',     shiftKey: false },
    { expected: rulerScale.LENGTH_MAX, key: 'End',        shiftKey: false },
    { expected: rulerScale.LENGTH_MIN, key: 'Home',       shiftKey: false }
  ])('steps 88 by $key with shift $shiftKey to $expected', ({ expected, key, shiftKey }) => {
    expect(rulerScale.steppedLength(key, shiftKey, 88)).toBe(expected)
  })

  it.each(['PageDown', 'PageUp'])('holds the ten-line %s step against shift', key => {
    expect(rulerScale.steppedLength(key, true, 88)).toBe(rulerScale.steppedLength(key, false, 88))
  })

  it('declines a key the rail does not handle', () => {
    expect(rulerScale.steppedLength('Tab', false, 88)).toBeNull()
  })
})

describe('packLanes', () => {
  const lanes = (spans: Record<string, [number, number]>, width = 400) => Object.fromEntries(
    [...rulerScale.packLanes(new Map(Object.entries(spans).map(([key, [from, to]]) => [key, { from, to }])), width, 6)]
      .map(([key, tether]) => [key, tether.lane]))

  it('keeps tethers apart along the rail in the first lane', () => {
    expect(lanes({ code: [10, 30], docs: [40, 60] })).toStrictEqual({ code: 0, docs: 0 })
  })

  it('drops a tether that would overlap the one before it to the next lane', () => {
    expect(lanes({ code: [10, 50], docs: [40, 60], imports: [70, 80] })).toStrictEqual({ code: 0, docs: 1, imports: 0 })
  })

  it('drops a tether starting within the slop of the last end, read in pixels', () => {
    expect(lanes({ code: [10, 30], docs: [31, 40] })).toStrictEqual({ code: 0, docs: 1 })
    expect(lanes({ code: [10, 30], docs: [31, 40] }, 1000)).toStrictEqual({ code: 0, docs: 0 })
  })

  it('reads a rail of no width at the fallback width', () => {
    expect(lanes({ code: [10, 30], docs: [32, 40] }, 0)).toStrictEqual({ code: 0, docs: 0 })
  })
})
