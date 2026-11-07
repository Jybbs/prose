import { parse }                  from 'postcss'
import type { AtRule, Container } from 'postcss'

import { perScheme, SCHEMES } from '../shared/palette'
import type { Scheme }        from '../shared/palette'

const REFERENCE = 'var('

// Returns the scheme a block of custom properties applies in, meaning both for
// `:root` and a Tailwind `@theme` block and the dark scheme alone for `.dark`,
// reading no block a media query holds.
function scopeOf(block: Container | undefined): readonly Scheme[] {
  if (block?.type === 'atrule') return (block as AtRule).name === 'theme' ? SCHEMES : []
  if (block?.type !== 'rule' || block.parent?.type === 'atrule' && (block.parent as AtRule).name === 'media') return []
  const { selector } = block as Container & { selector: string }
  return selector === ':root' ? SCHEMES : selector === '.dark' ? ['dark'] : []
}

// Holds the custom properties a set of stylesheets declares in each scheme, as
// the root element carries them once the sheets load in order, so a later
// declaration replaces an earlier one at the same specificity.
export class Cascade {
  private constructor(private readonly schemes: Record<Scheme, ReadonlyMap<string, string>>) {}

  static read(sheets: readonly string[]): Cascade {
    const schemes = perScheme(() => new Map<string, string>())
    for (const css of sheets) {
      parse(css).walkDecls(/^--/, decl => {
        for (const scheme of scopeOf(decl.parent)) schemes[scheme].set(decl.prop, decl.value.trim())
      })
    }
    return new Cascade(schemes)
  }

  // Returns `value` with each `var()` replaced by what its property holds in
  // `scheme`, or by its fallback where the property is undeclared, so the
  // result reads no custom property this cascade declares.
  resolve(value: string, scheme: Scheme): string {
    const at = value.lastIndexOf(REFERENCE)
    if (at < 0) return value
    let depth = 1
    let end   = at + REFERENCE.length
    for (; depth > 0; end++) depth += value[end] === '(' ? 1 : value[end] === ')' ? -1 : 0
    const [name, ...fallback] = value.slice(at + REFERENCE.length, end - 1).split(',')
    const held = this.schemes[scheme].get(name!.trim()) ?? fallback.join(',').trim()
    return this.resolve(value.slice(0, at) + held + value.slice(end), scheme)
  }

  // Returns what `property` holds in each scheme once every reference resolves.
  value(property: string): Record<Scheme, string> {
    return perScheme(scheme => this.resolve(`var(${property})`, scheme))
  }
}
