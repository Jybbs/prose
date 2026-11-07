import * as rulerScale from '../../lib/sandbox/ruler-scale'

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
