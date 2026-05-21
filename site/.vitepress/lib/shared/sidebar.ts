import type { DefaultTheme } from 'vitepress'

import { type DiscoveredRule }                                                          from '../rules/discovery'
import { FAMILY_META, PRIMITIVES, PUBLIC_PRIMITIVES, type PrimitiveSlug, type RuleFamily } from './registries'

const FAMILY_ORDER: readonly RuleFamily[] = ['alignment', 'ordering', 'formatting', 'docs', 'lint']

const primLink = (text: string, slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/primitives/${slug}`, text })

const ruleLink = (slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/rules/${slug}`, text: slug })

const GUIDE_SIDEBAR: DefaultTheme.SidebarItem[] = [
  {
    items: [
      { link: '/guide/installation',        text: 'Installation'        },
      { link: '/guide/quick-start',         text: 'Quick Start'         },
      { link: '/guide/two-stage-pipeline',  text: 'Two-Stage Pipeline'  },
      { link: '/guide/suppression',         text: 'Suppression'         }
    ],
    text : 'Guide'
  }
]

const REFERENCE_SIDEBAR: DefaultTheme.SidebarItem[] = [
  {
    items: [
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
      { link: '/integrations/ruff',              text: 'Ruff'              },
      { link: '/integrations/editor',            text: 'Editor'            },
      { link: '/integrations/github-actions',    text: 'GitHub Actions'    },
      { link: '/integrations/pre-commit',        text: 'Pre-Commit'        },
      { link: '/integrations/shell-completions', text: 'Shell Completions' }
    ],
    text : 'Integrations'
  }
]

export function buildSidebar(rules: readonly DiscoveredRule[]): DefaultTheme.Sidebar {
  const familySections: DefaultTheme.SidebarItem[] = FAMILY_ORDER.map(family => ({
    collapsed : false,
    link      : `/rules/${family}/`,
    text      : FAMILY_META[family].label,
    items     : rules
      .filter(r => r.family === family)
      .map(r => ruleLink(r.slug))
  }))
  return {
    '/guide/'        : GUIDE_SIDEBAR,
    '/reference/'    : REFERENCE_SIDEBAR,
    '/integrations/' : INTEGRATIONS_SIDEBAR,
    '/primitives/'   : [
      { items: [{ link: '/primitives/', text: 'Overview' }], text: 'Primitives' },
      {
        collapsed : false,
        items     : PUBLIC_PRIMITIVES.map(slug => primLink(PRIMITIVES[slug], slug)),
        text      : 'Public Surface'
      },
      {
        collapsed : false,
        items     : (Object.keys(PRIMITIVES) as PrimitiveSlug[])
          .filter(slug => !PUBLIC_PRIMITIVES.includes(slug))
          .map(slug => primLink(PRIMITIVES[slug], slug)),
        text      : 'Crate Internal'
      }
    ],
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
