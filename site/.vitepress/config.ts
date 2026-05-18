import { defineConfig, type DefaultTheme }       from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin }                     from 'vitepress-plugin-tabs'

import { bodyLinkPlugin }                      from './lib/body-link-plugin'
import { REPO_URL, SITE_HOSTNAME }             from './lib/constants'
import { glossary }                            from './lib/glossary'
import { glossaryPlugin }                      from './lib/glossary-plugin'
import { repoRoot, rulesDir }                  from './lib/paths'
import { PRIMITIVES }                          from './lib/primitives'
import { ruleLinkPlugin }                      from './lib/rule-link'
import { discoverRuleFiles, splitByCategory }  from './lib/rules-discovery'
import { SHIKI_THEMES }                        from './lib/shiki'
import { readCargoVersion }                    from './lib/version'

const repoDir = repoRoot(import.meta.url)
const version = readCargoVersion(repoDir)

const primLink = (text: string, slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/primitives/${slug}`, text })

const ruleLink = (slug: string): DefaultTheme.SidebarItem =>
  ({ link: `/rules/${slug}`, text: slug })

const discoveredRules   = discoverRuleFiles(rulesDir(import.meta.url))
const { autoFix, lint } = splitByCategory(discoveredRules)
const validSlugs        = new Set(discoveredRules.map(r => r.slug))

const glossaryPhraseToSlug = new Map<string, string>()
for (const [slug, entry] of Object.entries(glossary)) {
  glossaryPhraseToSlug.set(slug, slug)
  for (const alias of entry.aliases ?? []) {
    glossaryPhraseToSlug.set(alias, slug)
  }
}

export default defineConfig({
  cleanUrls     : true,
  description   : 'A Python typesetter for the reader.',
  head          : [
    ['link', { href: '/favicon.svg',                 rel: 'icon',        type: 'image/svg+xml' }],
    ['link', { href: 'https://fonts.googleapis.com', rel: 'preconnect'                          }],
    ['link', { crossorigin: '', href: 'https://fonts.gstatic.com', rel: 'preconnect'            }],
    ['link', {
      href: 'https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,400;0,9..144,500;0,9..144,600;0,9..144,700;1,9..144,400;1,9..144,500;1,9..144,600&family=JetBrains+Mono:ital,wght@0,400;0,500;0,700;1,400;1,500&family=Lora:ital,wght@0,400;0,500;0,600;0,700;1,400;1,500&display=swap',
      rel : 'stylesheet'
    }]
  ],
  lastUpdated   : true,
  markdown      : {
    config     : md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
      md.use(ruleLinkPlugin(validSlugs))
      md.use(glossaryPlugin(glossaryPhraseToSlug))
      md.use(bodyLinkPlugin)
    },
    lineNumbers: false,
    theme      : SHIKI_THEMES
  },
  sitemap       : {
    hostname: SITE_HOSTNAME
  },
  themeConfig   : {
    editLink   : {
      pattern: `${REPO_URL}/edit/main/site/:path`,
      text   : 'Suggest an edit to this page'
    },
    footer     : {
      copyright: '© Jybbs',
      message  : 'Released under the MIT License.'
    },
    logo       : { alt: 'prose', src: '/logo.svg' },
    nav        : [
      { activeMatch: '/guide/',      link: '/guide/installation',  text: 'Guide'       },
      { activeMatch: '/primitives/', link: '/primitives/source',   text: 'Primitives'  },
      { activeMatch: '/rules/',      link: '/rules/',              text: 'Rules'       },
      {                              link: `${REPO_URL}/releases`, text: `v${version}` }
    ],
    outline    : { level: [2, 3] },
    search     : { provider: 'local' },
    sidebar    : {
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
    },
    siteTitle  : 'Prose',
    socialLinks: [
      { icon: 'github', link: REPO_URL }
    ]
  },
  title         : 'Prose',
  titleTemplate : ':title · Prose',
  transformPageData(pageData) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { href: `${SITE_HOSTNAME}/${pageData.relativePath.replace(/(^|\/)index\.md$/, '$1').replace(/\.md$/, '')}`, rel: 'canonical' }
    ])
  },
  vite          : {
    plugins: [groupIconVitePlugin() as never]
  }
})
