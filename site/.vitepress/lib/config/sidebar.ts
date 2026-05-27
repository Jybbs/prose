import type { DefaultTheme } from 'vitepress'

import { type DiscoveredPrimitive }                   from '../primitives/discovery'
import { type DiscoveredRule }                        from '../rules/discovery'
import { FAMILY_META, FAMILY_ORDER, type RuleFamily } from '../shared/registries'

const primLink = (slug: string, text: string): DefaultTheme.SidebarItem =>
  ({ link: `/primitives/${slug}`, text })

const ruleLink = (slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/rules/${slug}`, text: slug })

const USAGE_SIDEBAR: DefaultTheme.SidebarItem[] = [
  {
    items: [
      { link: '/usage/',             text: 'Overview'     },
      { link: '/usage/installation', text: 'Installation' },
      { link: '/usage/quick-start',  text: 'Quick Start'  },
      { link: '/usage/suppression',  text: 'Suppression'  }
    ],
    text : 'Usage'
  }
]

const REFERENCE_SIDEBAR: DefaultTheme.SidebarItem[] = [
  {
    items: [
      { link: '/reference/',                       text: 'Overview'               },
      { link: '/reference/cli',                    text: 'CLI'                    },
      { link: '/reference/exit-codes',             text: 'Exit Codes'             },
      { link: '/reference/output-formats',         text: 'Output Formats'         },
      { link: '/reference/configuration',          text: 'Configuration'          },
      { link: '/reference/suppression-directives', text: 'Suppression Directives' },
      { link: '/reference/pipeline-order',         text: 'Pipeline Order'         },
      { link: '/reference/glossary',               text: 'Glossary'               }
    ],
    text : 'Reference'
  }
]

const INTEGRATIONS_SIDEBAR: DefaultTheme.SidebarItem[] = [
  {
    items: [
      { link: '/integrations/',                  text: 'Overview'          },
      { link: '/integrations/editor',            text: 'Editor'            },
      { link: '/integrations/github-actions',    text: 'GitHub Actions'    },
      { link: '/integrations/pre-commit',        text: 'Pre-Commit'        },
      { link: '/integrations/ruff',              text: 'Ruff'              },
      { link: '/integrations/shell-completions', text: 'Shell Completions' }
    ],
    text : 'Integrations'
  }
]

export function buildSidebar(
  rules      : readonly DiscoveredRule[],
  primitives : readonly DiscoveredPrimitive[]
): DefaultTheme.Sidebar {
  const familySections: DefaultTheme.SidebarItem[] = FAMILY_ORDER.map(family => ({
    items : rules
      .filter(r => r.family === family)
      .map(r => ruleLink(r.slug)),
    link  : `/rules/${family}/`,
    text  : FAMILY_META[family].label
  }))
  const publicPrimitives   = primitives.filter(p => p.stability === 'public')
  const internalPrimitives = primitives.filter(p => p.stability === 'internal')
  return {
    '/integrations/' : INTEGRATIONS_SIDEBAR,
    '/usage/'        : USAGE_SIDEBAR,
    '/primitives/'   : [
      { items: [{ link: '/primitives/', text: 'Overview' }], text: 'Primitives' },
      {
        items : publicPrimitives.map(p => primLink(p.slug, p.name)),
        text  : 'Public Surface'
      },
      {
        items : internalPrimitives.map(p => primLink(p.slug, p.name)),
        text  : 'Crate Internal'
      }
    ],
    '/reference/'    : REFERENCE_SIDEBAR,
    '/rules/'        : [
      {
        items: [
          { link: '/rules/',             text: 'Overview'    },
          { link: '/rules/composition/', text: 'Composition' }
        ],
        text : 'Rules'
      },
      ...familySections
    ]
  }
}
