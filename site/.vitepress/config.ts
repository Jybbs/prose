import path from 'node:path'
import { fileURLToPath } from 'node:url'

import { defineConfig, type DefaultTheme } from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin } from 'vitepress-plugin-tabs'

import { discoverRules }    from './lib/rules'
import { readCargoVersion } from './lib/version'

const here     = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(here, '../..')
const rulesDir = path.resolve(here, '../rules')
const version  = readCargoVersion(repoRoot)

const primLink = (text: string, slug: string): DefaultTheme.SidebarItem =>
  ({ text, link: `/primitives/${slug}` })

const ruleLink = (slug: string): DefaultTheme.SidebarItem =>
  ({ text: slug, link: `/rules/${slug}` })

const rules   = discoverRules(rulesDir)
const autoFix = rules.filter(r => r.category === 'auto-fix').map(r => r.slug)
const lint    = rules.filter(r => r.category === 'lint'    ).map(r => r.slug)

export default defineConfig({
  cleanUrls     : true,
  description   : 'A Python typesetter for the reader.',
  head          : [
    ['link', { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.googleapis.com' }],
    ['link', { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: '' }],
    ['link', {
      rel : 'stylesheet',
      href: 'https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,400;0,9..144,500;0,9..144,600;0,9..144,700;1,9..144,400;1,9..144,500;1,9..144,600&family=JetBrains+Mono:ital,wght@0,400;0,500;0,700;1,400;1,500&family=Lora:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500&display=swap'
    }]
  ],
  lastUpdated   : true,
  markdown      : {
    config     : md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
    },
    lineNumbers: false,
    theme      : { light: 'github-light', dark: 'github-dark' }
  },
  sitemap       : {
    hostname: 'https://prose.pages.dev'
  },
  title         : 'Prose',
  titleTemplate : ':title · Prose',
  transformPageData(pageData) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { rel: 'canonical', href: `https://prose.pages.dev/${pageData.relativePath.replace(/(^|\/)index\.md$/, '$1').replace(/\.md$/, '')}` }
    ])
  },
  vite          : {
    plugins: [groupIconVitePlugin() as never]
  },
  themeConfig: {
    editLink   : {
      pattern: 'https://github.com/Jybbs/prose/edit/main/site/:path',
      text   : 'Suggest an edit to this page'
    },
    footer     : {
      copyright: '© Jybbs',
      message  : 'Released under the MIT License.'
    },
    logo       : { src: '/logo.svg', alt: 'prose' },
    nav        : [
      { text: 'Guide',       link: '/guide/installation', activeMatch: '/guide/'      },
      { text: 'Rules',       link: '/rules/',             activeMatch: '/rules/'      },
      { text: 'Primitives',  link: '/primitives/source',  activeMatch: '/primitives/' },
      { text: `v${version}`, link: 'https://github.com/Jybbs/prose/releases' }
    ],
    outline    : { level: [2, 3] },
    search     : { provider: 'local' },
    sidebar    : {
      '/guide/': [
        {
          text : 'Guide',
          items: [
            { text: 'Installation',       link: '/guide/installation'       },
            { text: 'Configuration',      link: '/guide/configuration'      },
            { text: 'Suppression',        link: '/guide/suppression'        },
            { text: 'Editor Integration', link: '/guide/editor-integration' },
            { text: 'CI Integration',     link: '/guide/ci-integration'     }
          ]
        }
      ],
      '/rules/': [
        { text: 'Rules',    items: [{ text: 'Overview', link: '/rules/' }] },
        { text: 'Auto-Fix', items: autoFix.map(ruleLink) },
        { text: 'Lint',     items: lint.map(ruleLink) }
      ],
      '/primitives/': [
        {
          text : 'Primitives',
          items: [
            primLink('Source',          'source'),
            primLink('Pipeline',        'pipeline'),
            primLink('BindingAnalysis', 'binding-analysis'),
            primLink('SuppressionMap',  'suppression-map'),
            primLink('RuleId',          'rule-id')
          ]
        }
      ]
    },
    siteTitle  : 'Prose',
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Jybbs/prose' }
    ]
  }
})
