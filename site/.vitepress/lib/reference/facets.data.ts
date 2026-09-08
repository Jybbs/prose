import { defineLoader } from 'vitepress'

import { inlineNodes, type InlineNode } from '../markdown/inline-nodes'
import { getRenderer }                  from '../markdown/renderer'
import { discoverRuleIndex }            from '../rules/discovery'
import * as paths                       from '../shared/paths'
import { FAMILY_META }                  from '../shared/registries'
import * as ruleSchema                  from '../shared/rule-schema'
import * as anchors                     from './anchors'

interface Facet {
  anchor       : string
  default      : string
  key          : string
  meaningNodes : InlineNode[]
  type         : string
}

interface RuleGroup {
  anchor : string
  facets : readonly Facet[]
  rule   : string
}

interface FacetFamily {
  badge  : string
  family : string
  label  : string
  rules  : readonly RuleGroup[]
}

const root = paths.repoRoot(import.meta.url)

const rulesDirectory = paths.rulesDir(import.meta.url)

declare const data: readonly FacetFamily[]
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root), `${rulesDirectory}/*/*.md`],
  async load(): Promise<readonly FacetFamily[]> {
    const md     = await getRenderer()
    const schema = ruleSchema.proseSchema(root)
    const index  = discoverRuleIndex(rulesDirectory)
    const defs   = schema.$defs
    const rules  = ruleSchema.ruleDefsOf(schema)

    const facet = (
      key   : string,
      prop  : ruleSchema.SchemaProp,
      rule  : string,
      value : unknown
    ): Facet => ({
      anchor       : anchors.facetAnchor(rule, key),
      default      : JSON.stringify(value),
      key          : key,
      meaningNodes : inlineNodes(md, prop.description ?? ''),
      type         : ruleSchema.typeOf(prop).replaceAll('`', '')
    })

    const scope = (key: string, prop: ruleSchema.SchemaProp, rule: string): RuleGroup => ({
      anchor : anchors.ruleAnchor(rule),
      facets : [facet(key, prop, rule, prop.default)],
      rule   : rule
    })

    // Holds the two facets the shared sub-tables give every rule and every
    // alignment rule, each rendered once under the scope that names it.
    const generic: FacetFamily = {
      badge  : '',
      family : 'generic',
      label  : 'Generic',
      rules  : Object.entries(anchors.HOISTED_SCOPES).map(([key, name]) =>
        scope(key, ruleSchema.hoistedProps(defs)[key], name))
    }

    const rows = Object.entries(rules)
      .toSorted(([a], [b]) => a.localeCompare(b))
      .flatMap(([slug, def]) => {
        const props  = ruleSchema.rulePropsOf(defs, def)
        const facets = ruleSchema.ownFacetKeys(defs, def.default)
          .map(key => facet(key, props[key], slug, def.default[key]))

        const family = index.get(slug)?.family
        return facets.length === 0 || family === undefined
          ? []
          : [{ anchor: anchors.ruleAnchor(slug), facets, family, rule: slug }]
      })

    return [
      generic,
      ...[...Map.groupBy(rows, row => row.family)]
        .toSorted(([a], [b]) => a.localeCompare(b))
        .map(([family, ruleGroups]): FacetFamily => ({
          badge  : FAMILY_META[family].badge,
          family : family,
          label  : FAMILY_META[family].label,
          rules  : ruleGroups.map(({ anchor, facets, rule }) => ({ anchor, facets, rule }))
        }))
    ]
  }
})
