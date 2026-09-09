import * as anchors         from '../reference/anchors'
import * as ruleSchema      from '../shared/rule-schema'
import type { TokenSource } from './sources'

const CONFIG_PAGE    = '/reference/configuration'
const CATALOGUE_HREF = `${CONFIG_PAGE}#per-rule-facets`

const SECTION_HREF: Record<ruleSchema.ConfigSection, string> = {
  cache   : '/reference/cache#configuration',
  imports : `${CONFIG_PAGE}#imports`,
  top     : `${CONFIG_PAGE}#top-level-keys`
}

const TOP_HREF_OVERRIDES: Record<string, string> = {
  'docstring-line-length'       : `${CONFIG_PAGE}#docstring-budgets`,
  'docstring-structured-policy' : `${CONFIG_PAGE}#docstring-budgets`
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

// Points a facet at its own entry in the catalogue, falling back to the
// catalogue itself for one that several rules declare.
function facetHref(key: string, owners: Set<string>): string {
  const hoisted = anchors.hoistedFacetAnchor(key)
  if (hoisted !== undefined) return `${CONFIG_PAGE}#${hoisted}`
  if (owners.size === 1)     return `${CONFIG_PAGE}#${anchors.facetAnchor([...owners][0], key)}`
  return CATALOGUE_HREF
}

export function configKeySources(schema: ruleSchema.SchemaDocument): TokenSource[] {
  const sections  = ruleSchema.sectionProps(schema)
  const sectioned = (Object.entries(sections) as [ruleSchema.ConfigSection, ruleSchema.SchemaProps][])
    .flatMap(([section, props]) => Object.entries(props).map(([key, prop]) => ({
      key   : section === 'top' ? key : `${section}.${key}`,
      href  : section === 'top' ? TOP_HREF_OVERRIDES[key] ?? SECTION_HREF.top : SECTION_HREF[section],
      blurb : firstSentence(prop.description ?? '')
    })))

  const defs   = schema.$defs
  const facets = new Map<string, { owners: Set<string>, seen: Set<string> }>()
  for (const [slug, def] of Object.entries(ruleSchema.ruleDefsOf(schema))) {
    const props = ruleSchema.facetPropsOf(defs, def)
    for (const key of ruleSchema.facetKeys(def.default)) {
      const prop  = props[key]
      const entry = facets.get(key) ?? { owners: new Set<string>(), seen: new Set<string>() }
      entry.owners.add(slug)
      entry.seen.add(prop?.description ?? '')
      facets.set(key, entry)
    }
  }
  const facetSources = [...facets].map(([key, { owners, seen }]) => {
    const blurb = seen.size === 1 ? firstSentence([...seen][0]) : SHARED_FACET_BLURBS[key]
    if (blurb === undefined) {
      throw new Error(`facet ${key} carries several descriptions and no shared blurb`)
    }
    return { key, href: facetHref(key, owners), blurb }
  })

  return [...sectioned, ...facetSources, ...UNSCHEMED]
}
