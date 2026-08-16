import { defineLoader } from 'vitepress'

import { inlineNodes, type InlineNode } from '../markdown/inline-nodes'
import { getRenderer }                  from '../markdown/renderer'
import { discoverRuleIndex }            from '../rules/discovery'
import * as paths                       from '../shared/paths'
import { FAMILY_META, type RuleFamily } from '../shared/registries'
import * as ruleSchema                  from '../shared/rule-schema'

interface Facet {
  default      : string
  key          : string
  meaningNodes : InlineNode[]
  type         : string
}

interface RuleGroup {
  facets : readonly Facet[]
  rule   : string
}

interface FacetFamily {
  badge  : string
  family : string
  label  : string
  rules  : readonly RuleGroup[]
}

const ALIGNMENT_SCOPE = 'alignment rules'
const EVERY_RULE      = 'every rule'

const root = paths.repoRoot(import.meta.url)

declare const data: readonly FacetFamily[]
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<readonly FacetFamily[]> {
    const md     = await getRenderer()
    const schema = ruleSchema.proseSchema(root)
    const index  = discoverRuleIndex(paths.rulesDir(import.meta.url))
    const defs   = schema.$defs
    const rules  = defs.RuleConfigs.properties as Record<string, ruleSchema.RuleDef>

    const facet = (
      key     : string,
      prop    : ruleSchema.SchemaProp,
      value   : unknown,
      meaning : string
    ): Facet => ({
      default      : JSON.stringify(value),
      key          : key,
      meaningNodes : inlineNodes(md, meaning),
      type         : ruleSchema.typeOf(prop).replaceAll('`', '')
    })

    const describe = (props: ruleSchema.SchemaProps, key: string): string =>
      props[key].description ?? ''

    // `enabled` and `max-shift` repeat across every rule and every alignment
    // rule, so they read once as a scope rather than per rule.
    const aligner = Object.entries(rules).find(([, def]) => 'max-shift' in def.default)!
    const generic: FacetFamily = {
      badge  : '',
      family : 'generic',
      label  : 'Generic',
      rules  : [
        {
          rule   : EVERY_RULE,
          facets : [facet(
            'enabled',
            defs.ToggleOnly.properties.enabled,
            true,
            describe(defs.ToggleOnly.properties, 'enabled')
          )]
        },
        {
          rule   : ALIGNMENT_SCOPE,
          facets : [facet(
            'max-shift',
            defs.AlignmentConfig.properties['max-shift'],
            aligner[1].default['max-shift'],
            describe(defs.AlignmentConfig.properties, 'max-shift')
          )]
        }
      ]
    }

    // The generic group carries each hoisted facet once, so its keys are what
    // the per-rule lists drop.
    const hoisted = new Set(generic.rules.flatMap(group => group.facets.map(one => one.key)))

    const groups = new Map<RuleFamily, RuleGroup[]>()
    for (const [slug, def] of Object.entries(rules).toSorted(([a], [b]) => a.localeCompare(b))) {
      const props  = ruleSchema.rulePropsOf(defs, def)
      const facets = Object.keys(def.default)
        .filter(key => !hoisted.has(key))
        .toSorted((a, b) => a.localeCompare(b))
        .map(key => facet(key, props[key], def.default[key], describe(props, key)))

      const family = index.get(slug)?.family
      if (facets.length === 0 || family === undefined) continue
      groups.set(family, [...(groups.get(family) ?? []), { facets, rule: slug }])
    }

    return [
      generic,
      ...[...groups.entries()]
        .toSorted(([a], [b]) => a.localeCompare(b))
        .map(([family, ruleGroups]): FacetFamily => ({
          badge  : FAMILY_META[family].badge,
          family : family,
          label  : FAMILY_META[family].label,
          rules  : ruleGroups
        }))
    ]
  }
})
