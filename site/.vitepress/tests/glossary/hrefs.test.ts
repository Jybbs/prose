import type { GlossaryEntry }       from '../../lib/glossary/entries'
import { entryHref, glossaryHrefs } from '../../lib/glossary/hrefs'

const rules = new Map([['align-equals', { href: '/rules/alignment/align-equals' }]])

const entry = (overrides: Partial<GlossaryEntry>): GlossaryEntry =>
  ({ definition: 'd', families: ['engine'], ...overrides }) as GlossaryEntry

describe('entryHref', () => {
  it('resolves a rule-backed entry through the rule index', () => {
    expect(entryHref('x', entry({ rule: 'align-equals' }), rules)).toBe('/rules/alignment/align-equals')
  })

  it('throws when the entry names an unknown rule', () => {
    expect(() => entryHref('x', entry({ rule: 'ghost' }), rules)).toThrow(/unknown rule/)
  })

  it('throws on a hand-written rule URL', () => {
    expect(() => entryHref('x', entry({ href: '/rules/alignment/align-equals' }), rules))
      .toThrow(/rule field/)
  })

  it('passes a plain href through', () => {
    expect(entryHref('x', entry({ href: '/reference/cache' }), rules)).toBe('/reference/cache')
  })

  it('returns undefined for an unlinked entry', () => {
    expect(entryHref('x', entry({}), rules)).toBeUndefined()
  })
})

describe('glossaryHrefs', () => {
  it('maps only the entries that resolve to an href', () => {
    const map = glossaryHrefs({ linked: entry({ href: '/reference/cache' }), plain: entry({}) }, rules)
    expect([...map]).toEqual([['linked', '/reference/cache']])
  })
})
