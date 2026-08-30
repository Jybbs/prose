import { defineLoader, type MarkdownRenderer } from 'vitepress'

import { getRenderer, renderPlainInlineHtml } from '../markdown/renderer'
import { discoverRuleIndex }                  from '../rules/discovery'
import * as paths                             from '../shared/paths'
import type { RuleFamily }                    from '../shared/registries'
import * as ruleSchema                        from '../shared/rule-schema'
import { stripSuffix }                        from '../shared/strip-suffix'
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
  props    : ruleSchema.SchemaProps
): Facet[] {
  const keys = ruleSchema.facetKeys(defaults)
  return keys.map(key => ({
    default  : defaults[key] as Facet['default'],
    hintHtml : renderPlainInlineHtml(md, props[key]?.description ?? ''),
    key,
    kind     : facetKind(defaults[key]),
    label    : toTitleCase(key, '-')
  }))
}

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  load  : async (): Promise<SandboxSchema> => {
    const md       = await getRenderer()
    const schema   = ruleSchema.proseSchema(root)
    const defs     = schema.$defs
    const index    = discoverRuleIndex(paths.rulesDir(import.meta.url))
    const ruleDefs = ruleSchema.ruleDefsOf(schema)
    const rules    = Object.entries(ruleDefs).map(([slug, def]): RuleControl => ({
      facets : facetsOf(def.default, md, ruleSchema.rulePropsOf(defs, def)),
      family : (index.get(slug)?.family ?? '') as RuleFamily | '',
      slug
    }))
    const props   = schema.properties
    const lengths = Object.entries(props)
      .filter(([key, def]) => key.endsWith('-line-length') && typeof def.default === 'number')
      .map(([key, def]): LengthKnob => ({
        default : def.default as number,
        key,
        label   : toTitleCase(stripSuffix(key, '-line-length'), '-')
      }))
    return { codeLineLength: props['code-line-length'].default as number, lengths, rules }
  }
})
