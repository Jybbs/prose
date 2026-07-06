import * as registries from '../../lib/shared/registries'

describe('categoryOf', () => {
  it.each([
    ['alignment',  'auto-fix'],
    ['docs',       'auto-fix'],
    ['formatting', 'auto-fix'],
    ['layout',     'auto-fix'],
    ['lint',       'lint'],
    ['ordering',   'auto-fix']
  ] as const)('maps %s to %s', (family, expected) => {
    expect(registries.categoryOf(family)).toBe(expected)
  })
})

describe('GLOSSARY_FAMILY_META', () => {
  it.each([...registries.FAMILY_ORDER])('carries %s unchanged from registries.FAMILY_META', (family) => {
    expect(registries.GLOSSARY_FAMILY_META[family]).toMatchObject(registries.FAMILY_META[family])
  })
})

describe('registry types', () => {
  it('categoryOf returns a RuleCategory', () => {
    expectTypeOf(registries.categoryOf).returns.toEqualTypeOf<registries.RuleCategory>()
  })

  it('FAMILY_META keys equal the RuleFamily union', () => {
    expectTypeOf<keyof typeof registries.FAMILY_META>().toEqualTypeOf<registries.RuleFamily>()
  })

  it('GLOSSARY_FAMILY_META keys equal the GlossaryFamily union', () => {
    expectTypeOf<keyof typeof registries.GLOSSARY_FAMILY_META>().toEqualTypeOf<registries.GlossaryFamily>()
  })
})
