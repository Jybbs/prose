import type { DefaultTheme } from 'vitepress'

import { splitByCategory, type DiscoveredRule } from './rules/discovery'
import { PRIMITIVES } from './shared/registries'

const primLink = (text: string, slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/primitives/${slug}`, text })

const ruleLink = (slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/rules/${slug}`, text: slug })

export function buildSidebar(rules: readonly DiscoveredRule[]): DefaultTheme.Sidebar {
  const { autoFix, lint } = splitByCategory(rules)
  return {
    '/guide/': [
      {
        items: [
          { link: '/guide/installation',       text: 'Installation'       },
          { link: '/guide/configuration',      text: 'Configuration'      },
          { link: '/guide/suppression',        text: 'Suppression'        },
          { link: '/guide/editor-integration', text: 'Editor Integration' },
          { link: '/guide/ci-integration',     text: 'CI Integration'     }
        ],
        text : 'Guide'
      }
    ],
    '/primitives/': [
      {
        items: Object.entries(PRIMITIVES).map(([slug, label]) => primLink(label, slug)),
        text : 'Primitives'
      }
    ],
    '/rules/': [
      { items: [{ link: '/rules/', text: 'Overview' }], text: 'Rules'    },
      { items: autoFix.map(ruleLink),                   text: 'Auto-Fix' },
      { items: lint.map(ruleLink),                      text: 'Lint'     }
    ]
  }
}
