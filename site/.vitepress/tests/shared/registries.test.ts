import * as registries from '../../lib/shared/registries'

describe('sectionOf', () => {
  it.each([
    ['index.md',                        null],
    ['prototypes/icons/index.md',       null],
    ['reference/cli.md',                'reference'],
    ['rules/alignment/align-equals.md', 'rules'],
    ['sandbox/index.md',                'sandbox']
  ] as const)('maps %s to %s', (rel, expected) => {
    expect(registries.sectionOf(rel)).toBe(expected)
  })
})

describe('GLOSSARY_FAMILY_META', () => {
  it.each([...registries.FAMILY_ORDER])('carries %s unchanged from registries.FAMILY_META', (family) => {
    expect(registries.GLOSSARY_FAMILY_META[family]).toMatchObject(registries.FAMILY_META[family])
  })
})

describe('registry types', () => {
  it('FAMILY_META keys equal the RuleFamily union', () => {
    expectTypeOf<keyof typeof registries.FAMILY_META>().toEqualTypeOf<registries.RuleFamily>()
  })

  it('GLOSSARY_FAMILY_META keys equal the GlossaryFamily union', () => {
    expectTypeOf<keyof typeof registries.GLOSSARY_FAMILY_META>().toEqualTypeOf<registries.GlossaryFamily>()
  })
})
