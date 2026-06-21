import {
  categoryOf, FAMILY_META, FAMILY_ORDER, GLOSSARY_FAMILY_META
} from '../lib/shared/registries'
import type { GlossaryFamily, RuleCategory, RuleFamily } from '../lib/shared/registries'

describe('categoryOf', () => {
  it.each([
    ['alignment',  'auto-fix'],
    ['docs',       'auto-fix'],
    ['formatting', 'auto-fix'],
    ['layout',     'auto-fix'],
    ['lint',       'lint'],
    ['ordering',   'auto-fix']
  ] as const)('maps %s to %s', (family, expected) => {
    expect(categoryOf(family)).toBe(expected)
  })
})

describe('GLOSSARY_FAMILY_META', () => {
  it.each([...FAMILY_ORDER])('carries %s unchanged from FAMILY_META', (family) => {
    expect(GLOSSARY_FAMILY_META[family]).toMatchObject(FAMILY_META[family])
  })
})

describe('registry types', () => {
  it('categoryOf returns a RuleCategory', () => {
    expectTypeOf(categoryOf).returns.toEqualTypeOf<RuleCategory>()
  })

  it('FAMILY_META keys equal the RuleFamily union', () => {
    expectTypeOf<keyof typeof FAMILY_META>().toEqualTypeOf<RuleFamily>()
  })

  it('GLOSSARY_FAMILY_META keys equal the GlossaryFamily union', () => {
    expectTypeOf<keyof typeof GLOSSARY_FAMILY_META>().toEqualTypeOf<GlossaryFamily>()
  })
})
