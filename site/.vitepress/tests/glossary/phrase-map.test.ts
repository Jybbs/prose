import { fc, test } from '@fast-check/vitest'

import type { GlossaryEntry } from '../../lib/glossary/entries'
import { buildPhraseToSlug }  from '../../lib/glossary/phrase-map'

const entry = (overrides: Partial<GlossaryEntry> = {}): GlossaryEntry =>
  ({ definition: 'x', families: ['lint'], ...overrides })

describe('buildPhraseToSlug', () => {
  it('maps each slug and alias to its slug', () => {
    const map = buildPhraseToSlug({
      'atomic':        entry({ aliases: ['atom', 'atoms'] }),
      'count trigger': entry()
    })
    expect([...map.entries()].sort()).toEqual([
      ['atom', 'atomic'],
      ['atomic', 'atomic'],
      ['atoms', 'atomic'],
      ['count trigger', 'count trigger']
    ])
  })

  it('throws when a phrase is claimed by two slugs', () => {
    expect(() => buildPhraseToSlug({
      one: entry({ aliases: ['shared'] }),
      two: entry({ aliases: ['shared'] })
    })).toThrow(/registered against both/)
  })

  test.prop([fc.uniqueArray(fc.string({ minLength: 1, maxLength: 8 }), { minLength: 1 })])(
    'maps every alias-free slug to itself',
    (slugs) => {
      const map = buildPhraseToSlug(Object.fromEntries(slugs.map(s => [s, entry()])))
      for (const s of slugs) expect(map.get(s)).toBe(s)
    }
  )
})
