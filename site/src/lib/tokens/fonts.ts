const SERIF_FALLBACKS = 'Georgia, "Times New Roman", serif'
const MONO_FALLBACKS  = 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'

export const FONTS = {
  base    : { fallbacks: SERIF_FALLBACKS, name: 'Lora',           slug: 'lora',           staticWeights: [400],      weightSpan: '400 700' },
  display : { fallbacks: SERIF_FALLBACKS, name: 'Fraunces',       slug: 'fraunces',       staticWeights: [600],      weightSpan: '100 900' },
  mono    : { fallbacks: MONO_FALLBACKS,  name: 'JetBrains Mono', slug: 'jetbrains-mono', staticWeights: [500, 700], weightSpan: '100 800' }
} as const

export const FONT_FAMILIES = Object.values(FONTS).map(face => ({
  cssVariable : `--font-${face.slug}` as const,
  name        : `${face.name} Variable`,
  options     : { package: `@fontsource-variable/${face.slug}` },
  weights     : [face.weightSpan] as [string]
}))

export const fontStack = ({ fallbacks, name }: (typeof FONTS)[keyof typeof FONTS]): string =>
  `'${name} Variable', ${fallbacks}`
