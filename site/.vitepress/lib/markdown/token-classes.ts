import { applyColorReplacements, bundledThemes, normalizeTheme, type ShikiTransformer } from 'shiki'

import { SHIKI_COLOR_REPLACEMENTS, SHIKI_THEMES } from '../shared/constants'

type Scheme = keyof typeof SHIKI_THEMES

const PREFIXES: Record<Scheme, string> = { dark: 'sd', light: 'sl' }

// Lists each property and value a token's `FontStyle` bits write, under the
// `--shiki-<scheme>-<property>` name a dual-theme highlight gives each one.
const FONT_STYLES = [
  ['font-style',      'italic'],
  ['font-weight',     'bold'],
  ['text-decoration', 'underline'],
  ['text-decoration', 'line-through'],
  ['text-decoration', 'underline line-through']
] as const

let shared: Promise<TokenClasses> | undefined

// Maps each `--shiki-*` declaration a token span can carry under the site's two
// themes to a class setting that one property, so a highlighted block names a
// class on each span where it would repeat the declarations inline.
export class TokenClasses {
  private constructor(private readonly classes: ReadonlyMap<string, string>) {}

  // Reads every foreground of both themes after the site's color replacements,
  // returning one instance however many modules ask for it.
  static read(): Promise<TokenClasses> {
    return shared ??= Promise.all(Object.entries(SHIKI_THEMES).map(async ([scheme, name]) => {
      const theme  = normalizeTheme((await bundledThemes[name]()).default)
      const colors = [theme.fg, ...theme.settings.map(({ settings }) => settings.foreground)]
        .filter(color => color !== undefined)
        .map(color => applyColorReplacements(color, SHIKI_COLOR_REPLACEMENTS[name]).toLowerCase())
      const prefix = PREFIXES[scheme as Scheme]
      return [
        ...colors.map(color => [`--shiki-${scheme}:${color}`, `${prefix}-${color.slice(1)}`] as const),
        ...FONT_STYLES.map(([property, value]) =>
          [`--shiki-${scheme}-${property}:${value}`, `${prefix}-${value.replace(' ', '-')}`] as const)
      ]
    })).then(entries => new TokenClasses(new Map(entries.flat())))
  }

  // Writes one rule for each class, setting the custom property the theme
  // stylesheet reads in each color scheme.
  css(): string {
    return [...this.classes].map(([declaration, name]) => `.${name} { ${declaration}; }\n`).join('')
  }

  // Moves each declaration of a token span's `style` that a class covers onto
  // that class, keeping the rest inline.
  transformer(): ShikiTransformer {
    const { classes } = this
    return {
      name: 'prose:token-classes',
      span(node) {
        const { style } = node.properties
        if (typeof style !== 'string') return
        const inline = style.split(';').filter(declaration => {
          const name = classes.get(declaration.toLowerCase())
          if (name !== undefined) this.addClassToHast(node, name)
          return declaration !== '' && name === undefined
        })
        if (inline.length > 0) node.properties.style = inline.join(';')
        else delete node.properties.style
      }
    }
  }
}
