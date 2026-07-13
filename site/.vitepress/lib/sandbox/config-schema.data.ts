import { defineLoader, type MarkdownRenderer } from 'vitepress'

import { getRenderer, renderPlainInlineHtml } from '../markdown/renderer'
import { discoverRuleIndex }                  from '../rules/discovery'
import * as paths                             from '../shared/paths'
import type { RuleFamily }                    from '../shared/registries'
import { toTitleCase }                        from '../shared/title-case'

type FacetKind = 'bool' | 'int' | 'string' | 'stringList'

export type FacetValue = boolean | number | string | readonly string[]

export interface Facet {
  default  : FacetValue
  hintHtml : string
  key      : string
  kind     : FacetKind
  label    : string
}

export interface RuleControl {
  facets : readonly Facet[]
  family : RuleFamily | ''
  slug   : string
}

export interface LengthKnob {
  default : number
  key     : string
  label   : string
}

export interface SandboxSchema {
  codeLineLength : number
  lengths        : readonly LengthKnob[]
  rules          : readonly RuleControl[]
}

type SchemaProps = Record<string, { description?: string }>

declare const data: SandboxSchema
export { data }

const root = paths.repoRoot(import.meta.url)

function facetKind(value: unknown): FacetKind {
  if (typeof value === 'boolean') return 'bool'
  if (typeof value === 'number')  return 'int'
  if (Array.isArray(value))       return 'stringList'
  return 'string'
}

function facetsOf(
  defaults : Record<string, unknown>,
  md       : MarkdownRenderer,
  props    : SchemaProps
): Facet[] {
  const keys = Object.keys(defaults).sort((a, b) =>
    a === 'enabled' ? -1 : b === 'enabled' ? 1 : a.localeCompare(b))
  return keys.map(key => ({
    default  : defaults[key] as Facet['default'],
    hintHtml : renderPlainInlineHtml(md, props[key]?.description ?? ''),
    key,
    kind     : facetKind(defaults[key]),
    label    : toTitleCase(key, '-')
  }))
}

export default defineLoader({
  watch : paths.proseBinaryCandidates(root),
  load  : async (): Promise<SandboxSchema> => {
    const md       = await getRenderer()
    const schema   = JSON.parse(paths.runProse(root, ['schema']))
    const index    = discoverRuleIndex(paths.rulesDir(import.meta.url))
    const ruleDefs = schema.$defs.RuleConfigs.properties as
      Record<string, { anyOf?: { $ref?: string }[], default: Record<string, unknown> }>
    const rules = Object.entries(ruleDefs).map(([slug, def]): RuleControl => {
      const ref   = def.anyOf?.map(entry => entry.$ref).find(Boolean)?.split('/').pop()
      const props = (ref ? schema.$defs[ref].properties : {}) as SchemaProps
      return {
        facets : facetsOf(def.default, md, props),
        family : (index.get(slug)?.family ?? '') as RuleFamily | '',
        slug
      }
    })
    const props   = schema.properties as Record<string, { default?: unknown }>
    const lengths = Object.entries(props)
      .filter(([key, def]) => key.endsWith('-line-length') && typeof def.default === 'number')
      .map(([key, def]): LengthKnob => ({
        default : def.default as number,
        key,
        label   : toTitleCase(key.replace(/-line-length$/, ''), '-')
      }))
    return { codeLineLength: props['code-line-length'].default as number, lengths, rules }
  }
})
