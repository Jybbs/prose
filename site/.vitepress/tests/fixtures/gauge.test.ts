import { changeValues, gaugePct, gaugeTicks } from '../../lib/fixtures/gauge'

describe('changeValues', () => {
  it('lists each value whose output differs from the value below it', () => {
    expect(changeValues([0, 0, 1, 1, 2])).toStrictEqual([2, 4])
  })

  it('lists no value where every output matches', () => {
    expect(changeValues([0, 0, 0])).toStrictEqual([])
  })
})

describe('gaugePct', () => {
  it('places the ends of the span at 0 and 100', () => {
    expect([gaugePct([0, 0, 0, 0, 0], 0), gaugePct([0, 0, 0, 0, 0], 4)]).toStrictEqual([0, 100])
  })
})

describe('gaugeTicks', () => {
  const svg = gaugeTicks([0, 0, 1])

  it('draws one tick per value across a box as wide as the span', () => {
    expect(svg.match(/<line /g)).toHaveLength(3)
    expect(svg).toContain('viewBox="0 0 2 10"')
  })

  it('draws a taller tick at each value where the output changes', () => {
    expect(svg).toContain('<line data-change="true" vector-effect="non-scaling-stroke" x1="2" x2="2" y1="0" y2="10"/>')
    expect(svg.match(/data-change="false"[^>]*y1="5"/g)).toHaveLength(2)
  })
})
