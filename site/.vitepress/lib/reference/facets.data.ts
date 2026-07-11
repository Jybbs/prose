import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineField } from '../markdown/renderer'
import { FAMILY_META, type RuleFamily }   from '../shared/registries'

interface Facet {
  default     : string
  key         : string
  meaningHtml : string
  type        : string
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

declare const data: readonly FacetFamily[]
export { data }

interface FacetSource {
  default : string
  key     : string
  meaning : string
  type    : string
}

interface RuleGroupSource {
  facets : readonly FacetSource[]
  rule   : string
}

interface FacetFamilySource {
  family : RuleFamily | 'generic'
  rules  : readonly RuleGroupSource[]
}

const SOURCES: readonly FacetFamilySource[] = [
  {
    family: 'generic',
    rules: [
      {
        rule   : 'every rule',
        facets : [
          {
            default : 'true',
            key     : 'enabled',
            meaning : 'Toggle the rule on or off.',
            type    : 'bool'
          }
        ]
      },
      {
        rule   : 'alignment rules',
        facets : [
          {
            default : '16',
            key     : 'max-shift',
            meaning : 'Width-spread budget for an alignment run. A positive `N` caps the spread, `0` forbids any '
                    + 'shift so every row sits flush, and `false` lifts the cap so a contiguous run folds into '
                    + 'one column. To leave one row out of an otherwise-aligned group, hold it with '
                    + '`# prose: skip`.',
            type    : 'positive int | 0 | false'
          }
        ]
      }
    ]
  },
  {
    family: 'ordering',
    rules: [
      {
        rule   : 'alphabetize',
        facets : [
          {
            default : 'true',
            key     : 'group-methods',
            meaning : 'Group methods into dunders, properties, privates, and publics before sorting within each '
                    + 'group. `false` sorts methods by plain name alone.',
            type    : 'bool'
          },
          {
            default : 'true',
            key     : 'sort-definitions',
            meaning : 'Reorder class and function definitions alphabetically, holding each behind any sibling it '
                    + 'names at evaluation time. `false` freezes definitions in source order while other '
                    + 'surfaces still sort.',
            type    : 'bool'
          },
          {
            default : 'true',
            key     : 'sort-docstring-entries',
            meaning : 'Reorder `name: description` entries within Title-case-headed docstring sections, parameter '
                    + 'entries mirroring the signature as the rule leaves it and stragglers alphabetizing below. '
                    + 'Set `false` to keep narrative-curated entry order while still sorting every other surface.',
            type    : 'bool'
          },
          {
            default : 'true',
            key     : 'sort-dunder-lists',
            meaning : 'Reorder the string items inside `__all__` and `__slots__`. `false` keeps a hand-curated '
                    + 'public-API order.',
            type    : 'bool'
          }
        ]
      }
    ]
  },
  {
    family: 'layout',
    rules: [
      {
        rule   : 'collection-layout',
        facets : [
          {
            default : 'true',
            key     : 'collapse',
            meaning : 'Join a fitting multi-line literal, subscript, or dict key back to one line. `false` '
                    + 'freezes those shapes where they sit.',
            type    : 'bool'
          },
          {
            default : 'true',
            key     : 'explode',
            meaning : 'Expand an overflowing or over-count collection to one entry per line. `false` suppresses '
                    + 'every expansion and leaves the count cap inert.',
            type    : 'bool'
          },
          {
            default : '8',
            key     : 'max-atomics',
            meaning : 'Keep short collections on one line when each entry is an atomic literal and the run fits '
                    + 'the cap. `false` removes the cap and packs by width alone.',
            type    : 'positive int | false'
          },
          {
            default : '3',
            key     : 'max-dict-entries',
            meaning : 'Expand a dict once its entry count exceeds the cap, whatever its width. `false` disables '
                    + 'the count trigger.',
            type    : 'positive int | false'
          },
          {
            default : 'true',
            key     : 'wrap-dict-entries',
            meaning : 'Break an over-wide `key: value` at its `:` and hang the value beneath. `false` leaves the '
                    + 'oversized entry on one line.',
            type    : 'bool'
          }
        ]
      },
      {
        rule   : 'call-layout',
        facets : [
          {
            default : '3',
            key     : 'max-args',
            meaning : 'Explode a call to one keyword argument per line once its argument count exceeds the cap. '
                    + '`false` disables the count trigger and leaves every call inline.',
            type    : 'positive int | false'
          }
        ]
      },
      {
        rule   : 'signature-layout',
        facets : [
          {
            default : '4',
            key     : 'max-params',
            meaning : 'Expand a signature to one parameter per line once its parameter count exceeds the cap. '
                    + '`false` disables the count trigger and leaves only the `code-line-length` budget.',
            type    : 'positive int | false'
          }
        ]
      }
    ]
  },
  {
    family: 'lint',
    rules: [
      {
        rule   : 'bare-imports',
        facets : [
          {
            default : '[]',
            key     : 'allow',
            meaning : 'Modules whose bare-import form is preserved whatever their attribute count.',
            type    : 'list of module names'
          },
          {
            default : 'true',
            key     : 'exempt-aliased',
            meaning : 'Exempt every aliased bare import (*`import x as y`*) from the rule.',
            type    : 'bool'
          },
          {
            default : '4',
            key     : 'max-attributes',
            meaning : 'Distinct-attribute count at or below which an unaliased bare import is flagged.',
            type    : 'positive int'
          }
        ]
      },
      {
        rule   : 'miscased-constants',
        facets : [
          {
            default : '""',
            key     : 'allow-pattern',
            meaning : 'Constant names exempted from the lint, such as old-style bare aliases.',
            type    : 'regex'
          }
        ]
      },
      {
        rule   : 'reassigned-constants',
        facets : [
          {
            default : '[]',
            key     : 'allow',
            meaning : 'Module-level names exempted from the lint.',
            type    : 'list of names'
          }
        ]
      },
      {
        rule   : 'single-use-variables',
        facets : [
          {
            default : '"^_"',
            key     : 'allow-pattern',
            meaning : 'Binding names exempted from the lint.',
            type    : 'regex'
          }
        ]
      }
    ]
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<readonly FacetFamily[]> {
    const md = await getRenderer()
    return SOURCES.map(family => ({
      badge  : family.family === 'generic' ? '' : FAMILY_META[family.family].badge,
      family : family.family,
      label  : family.family === 'generic' ? 'Generic' : FAMILY_META[family.family].label,
      rules  : family.rules.map(group => ({
        facets : renderInlineField(md, group.facets, 'meaning'),
        rule   : group.rule
      }))
    }))
  }
})
