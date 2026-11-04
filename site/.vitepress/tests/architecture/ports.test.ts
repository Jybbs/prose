import { PORT_WORDS, portsOf } from '../../lib/architecture/ports'

describe('portsOf', () => {
  it('tabs a consumed tile in, a consuming tile out, and a tile doing each both', () => {
    expect(portsOf({ consumedBy: ['walker', 'source'], consumes: ['source', 'ruff'] })).toStrictEqual(new Map([
      ['source', 'both'],
      ['ruff',   'in'],
      ['walker', 'out']
    ]))
  })

  it('tabs nothing with no primitive focused', () => {
    expect(portsOf(null).size).toBe(0)
  })

  it('names the both tab by each of its words', () => {
    expect(PORT_WORDS.both).toBe(`${PORT_WORDS.in} · ${PORT_WORDS.out}`)
  })
})
