import type { GlossaryEntry }                  from '../../lib/glossary/entries'
import { entryHref, entryRule, glossaryHrefs } from '../../lib/glossary/hrefs'

const rules = new Map([
  ['align-equals', { family: 'alignment' as const, href: '/rules/alignment/align-equals' }]
])

const entry = (overrides: Partial<GlossaryEntry>): GlossaryEntry =>
  ({ definition: 'd', families: ['engine'], ...overrides }) as GlossaryEntry

const resolve = (overrides: Partial<GlossaryEntry>) =>
  entryRule('x', entry(overrides), rules)

describe('entryRule', () => {
  it('resolves a rule-backed entry through the rule index', () => {
    expect(resolve({ rule: 'align-equals' })).toEqual(rules.get('align-equals'))
  })

  it('resolves to undefined for an entry naming no rule', () => {
    expect(resolve({})).toBeUndefined()
  })

  it('throws when the entry names an unknown rule', () => {
    expect(() => resolve({ rule: 'ghost' })).toThrow(/unknown rule/)
  })
})

describe('entryHref', () => {
  it('takes the href off the resolved rule', () => {
    expect(entryHref('x', entry({ rule: 'align-equals' }), resolve({ rule: 'align-equals' })))
      .toBe('/rules/alignment/align-equals')
  })

  it('throws on a hand-written rule URL', () => {
    expect(() => entryHref('x', entry({ href: '/rules/alignment/align-equals' })))
      .toThrow(/rule field/)
  })

  it('passes a plain href through', () => {
    expect(entryHref('x', entry({ href: '/reference/cache' }))).toBe('/reference/cache')
  })

  it('returns undefined for an unlinked entry', () => {
    expect(entryHref('x', entry({}))).toBeUndefined()
  })
})

describe('glossaryHrefs', () => {
  it('maps only the entries that resolve to an href', () => {
    const source = { linked: entry({ href: '/reference/cache' }), plain: entry({}) }
    expect([...glossaryHrefs(source, rules)]).toEqual([['linked', '/reference/cache']])
  })
})
