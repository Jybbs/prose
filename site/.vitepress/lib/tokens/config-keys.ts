import * as ruleSchema      from '../shared/rule-schema'
import type { TokenSource } from './sources'

const FACET_HREF = '/reference/configuration#per-rule-facets'

const SECTION_HREF: Record<ruleSchema.ConfigSection, string> = {
  cache   : '/reference/cache#configuration',
  imports : '/reference/configuration#imports',
  top     : '/reference/configuration#top-level-keys'
}

const TOP_HREF_OVERRIDES: Record<string, string> = {
  'docstring-line-length'       : '/reference/configuration#docstring-budgets',
  'docstring-structured-policy' : '/reference/configuration#docstring-budgets'
}

// A facet several rules describe differently takes one blurb here, whereas a
// facet with a single description across the rules takes that description.
const SHARED_FACET_BLURBS: Record<string, string> = {
  'allow'         : 'Per-rule exemption list, modules for `bare-imports` and names for `reassigned-constants`.',
  'allow-pattern' : 'Per-rule glob exempting matching names from a lint.',
  'max-shift'     : 'Per-rule padding limit for an alignment run or a hanging chain.'
}

const UNSCHEMED: readonly TokenSource[] = [
  {
    key   : 'overrides.paths',
    href  : '/reference/configuration#per-pattern-overrides',
    blurb : 'Glob list naming the files an override entry applies its partial config to.'
  }
]

export function firstSentence(text: string): string {
  let inCode = false
  for (let i = 0; i < text.length; i++) {
    const ch = text[i]
    if (ch === '`') {
      inCode = !inCode
    } else if (ch === '.' && !inCode && (i + 1 === text.length || /\s/.test(text[i + 1]))) {
      return text.slice(0, i + 1)
    }
  }
  return text
}

export function configKeySources(schema: ruleSchema.SchemaDocument): TokenSource[] {
  const sections  = ruleSchema.sectionProps(schema)
  const sectioned = (Object.entries(sections) as [ruleSchema.ConfigSection, ruleSchema.SchemaProps][])
    .flatMap(([section, props]) => Object.entries(props).map(([key, prop]) => ({
      key   : section === 'top' ? key : `${section}.${key}`,
      href  : section === 'top' ? TOP_HREF_OVERRIDES[key] ?? SECTION_HREF.top : SECTION_HREF[section],
      blurb : firstSentence(prop.description ?? '')
    })))

  const defs         = schema.$defs
  const descriptions = new Map<string, Set<string>>()
  for (const def of Object.values(ruleSchema.ruleDefsOf(schema))) {
    const props = ruleSchema.rulePropsOf(defs, def)
    for (const key of ruleSchema.facetKeys(def.default)) {
      const prop = key === 'enabled' ? defs.ToggleOnly.properties.enabled : props[key]
      descriptions.set(key, (descriptions.get(key) ?? new Set()).add(prop?.description ?? ''))
    }
  }
  const facets = [...descriptions].map(([key, seen]) => {
    const blurb = seen.size === 1 ? firstSentence([...seen][0]) : SHARED_FACET_BLURBS[key]
    if (blurb === undefined) {
      throw new Error(`facet ${key} carries several descriptions and no shared blurb`)
    }
    return { key, href: FACET_HREF, blurb }
  })

  return [...sectioned, ...facets, ...UNSCHEMED]
}
