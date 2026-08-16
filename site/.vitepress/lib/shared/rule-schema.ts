import { inlineNodes }                   from '../markdown/inline-nodes'
import type { InlineNode, InlineParser } from '../markdown/inline-nodes'
import { runProse }                      from './paths'

export const NESTED_TABLES = new Set(['cache', 'imports', 'rules'])

export interface ConfigRow {
  default      : string
  key          : string
  meaningNodes : InlineNode[]
  typeNodes    : InlineNode[]
}

// A rule's schema entry, the `anyOf` naming its sub-table and `default`
// carrying the facet keys the rule ships with.
export interface RuleDef {
  anyOf   ?: readonly { $ref?: string }[]
  default  : Record<string, unknown>
}

export type SchemaDefs = Record<string, { properties: SchemaProps }>

export interface SchemaDocument {
  $defs      : SchemaDefs
  properties : SchemaProps
}

export interface SchemaProp {
  $ref        ?: string
  anyOf       ?: readonly SchemaProp[]
  const       ?: unknown
  default     ?: unknown
  description ?: string
  format      ?: string
  items       ?: SchemaProp
  type        ?: string | readonly string[]
}

export type SchemaProps = Record<string, SchemaProp>

export type Section = 'cache' | 'imports' | 'rules' | 'top'

export function configRow(
  md    : InlineParser,
  key   : string,
  prop  : SchemaProp,
  value : unknown
): ConfigRow {
  return {
    default      : value === null ? 'unset' : JSON.stringify(value),
    key          : key,
    meaningNodes : inlineNodes(md, prop.description ?? ''),
    typeNodes    : inlineNodes(md, typeOf(prop))
  }
}

// Every key `[tool.prose]` accepts, grouped by the section it sits under, with
// the rule facets deduplicated across the rules that share them.
export function declaredKeys(schema: SchemaDocument): Record<Section, string[]> {
  const rules  = schema.$defs.RuleConfigs.properties as Record<string, RuleDef>
  const facets = Object.values(rules).flatMap(def => Object.keys(rulePropsOf(schema.$defs, def)))

  return {
    cache   : Object.keys(schema.$defs.CacheConfig.properties),
    imports : Object.keys(schema.$defs.ImportsConfig.properties),
    rules   : [...new Set(facets)],
    top     : Object.keys(schema.properties).filter(key => !NESTED_TABLES.has(key))
  }
}

// A rule's facet keys in table order, `enabled` leading and the rest sorting
// by name.
export function facetKeys(defaults: Record<string, unknown>): string[] {
  return Object.keys(defaults).toSorted((a, b) =>
    a === 'enabled' ? -1 : b === 'enabled' ? 1 : a.localeCompare(b))
}

// The configuration schema the `prose schema` subcommand prints.
export function proseSchema(root: string): SchemaDocument {
  return JSON.parse(runProse(root, ['schema'])) as SchemaDocument
}

// A rule's entry is `anyOf [bool, $ref]`, the ref naming the sub-table whose
// properties carry each facet's type and description.
export function rulePropsOf(defs: SchemaDefs, def: RuleDef): SchemaProps {
  const ref = def.anyOf?.map(entry => entry.$ref).find(Boolean)?.split('/').pop()
  return ref ? defs[ref].properties : {}
}

// The budget newtypes carry no `type`, so their `$ref` name is what names the
// accepted shape.
export function typeOf(prop: SchemaProp): string {
  const ref = prop.$ref?.split('/').pop()
  if (ref === 'DocstringStructuredPolicy') return '`"code-line-length"` | `"docstring-line-length"`'
  if (ref === 'InlineBudget')              return 'positive int | `false`'
  if (ref === 'MaxShift')                  return 'positive int | `0` | `false`'
  if (ref === 'PythonVersion')             return '`"3.X"` version string'

  if (prop.anyOf) {
    const shapes = prop.anyOf.filter(entry => entry.type !== 'null')
    if (shapes.some(entry => entry.const === false)) return 'positive int | `false`'
    if (shapes.length === 1) return typeOf(shapes[0])
  }

  const kinds = new Set([prop.type].flat())
  if (prop.format === 'regex')       return 'regex'
  if (prop.items?.type === 'string') return 'list of names'
  if (kinds.has('boolean'))          return 'bool'
  if (kinds.has('integer'))          return 'positive int'
  return 'string'
}
