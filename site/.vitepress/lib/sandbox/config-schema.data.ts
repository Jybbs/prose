import { execFileSync } from 'node:child_process'

import { defineLoader } from 'vitepress'

import { discoverRuleIndex }              from '../rules/discovery'
import { getRenderer, renderInlineHtml }  from '../markdown/renderer'
import type { MarkdownRenderer }          from 'vitepress'
import * as paths                         from '../shared/paths'
import type { RuleFamily }                from '../shared/registries'
import { toTitleCase }                    from '../shared/title-case'

export type FacetKind = 'bool' | 'int' | 'string' | 'stringList'

export interface Facet {
  default  : boolean | number | string | readonly string[]
  hint     : string
  hintHtml : string
  key      : string
  kind     : FacetKind
  label    : string
}

export interface RuleControl {
  facets : readonly Facet[]
  family : RuleFamily | ''
  hint   : string
  label  : string
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

type SchemaProps = Record<string, { description?: string }>

function facetsOf(
  defaults : Record<string, unknown>,
  md       : MarkdownRenderer,
  props    : SchemaProps
): Facet[] {
  const keys = Object.keys(defaults).sort((a, b) =>
    a === 'enabled' ? -1 : b === 'enabled' ? 1 : a.localeCompare(b))
  return keys.map(key => {
    const hint = props[key]?.description ?? ''
    return {
      default  : defaults[key] as Facet['default'],
      hint,
      hintHtml : renderInlineHtml(md, hint),
      key,
      kind     : facetKind(defaults[key]),
      label    : toTitleCase(key, '-')
    }
  })
}

export default defineLoader({
  watch : paths.proseBinaryCandidates(root),
  load  : async (): Promise<SandboxSchema> => {
    const md       = await getRenderer()
    const binary   = paths.resolveProseBinary(root)
    const schema   = JSON.parse(execFileSync(binary, ['schema'], { encoding: 'utf8' }))
    const index    = discoverRuleIndex(paths.rulesDir(import.meta.url))
    const ruleDefs = schema.$defs.RuleConfigs.properties as
      Record<string, { anyOf?: { $ref?: string }[], default: Record<string, unknown> }>
    const rules = Object.entries(ruleDefs).map(([slug, def]): RuleControl => {
      const ref   = def.anyOf?.map(entry => entry.$ref).find(Boolean)?.split('/').pop()
      const props = (ref ? schema.$defs[ref].properties : {}) as SchemaProps
      return {
        facets : facetsOf(def.default, md, props),
        family : (index.get(slug)?.family ?? '') as RuleFamily | '',
        hint   : index.get(slug)?.caption ?? '',
        label  : toTitleCase(slug, '-'),
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
