const SERIF_FALLBACKS = ['Georgia', 'Times New Roman', 'serif']
const MONO_FALLBACKS  = ['ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco', 'Consolas', 'monospace']

export const FONTS = {
  base    : { fallbacks: SERIF_FALLBACKS, name: 'Lora',           slug: 'lora',           staticWeights: [400],      weightSpan: '400 700' },
  display : { fallbacks: SERIF_FALLBACKS, name: 'Fraunces',       slug: 'fraunces',       staticWeights: [600],      weightSpan: '100 900' },
  mono    : { fallbacks: MONO_FALLBACKS,  name: 'JetBrains Mono', slug: 'jetbrains-mono', staticWeights: [500, 700], weightSpan: '100 800' }
} as const

export const FONT_FAMILIES = Object.values(FONTS).map(face => ({
  cssVariable : `--font-${face.slug}` as const,
  fallbacks   : face.fallbacks,
  name        : `${face.name} Variable`,
  options     : { package: `@fontsource-variable/${face.slug}` },
  weights     : [face.weightSpan] as [string]
}))

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('FONT_FAMILIES', () => {
    test('derives one family per declared face', () => {
      expect(FONT_FAMILIES).toHaveLength(Object.values(FONTS).length)
    })

    test.each(Object.values(FONTS).map((face, index) => ({ name: face.slug, face, family: FONT_FAMILIES[index] })))(
      'derives the $name family from its face',
      ({ face, family }) => {
        expect(family.cssVariable).toBe(`--font-${face.slug}`)
        expect(family.name).toBe(`${face.name} Variable`)
        expect(family.options.package).toBe(`@fontsource-variable/${face.slug}`)
        expect(family.weights).toEqual([face.weightSpan])
        expect(family.fallbacks).toBe(face.fallbacks)
      }
    )
  })
}
