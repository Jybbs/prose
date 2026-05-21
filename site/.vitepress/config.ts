import postcssCustomMedia                         from 'postcss-custom-media'
import { defineConfig }                            from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin }                     from 'vitepress-plugin-tabs'

import { buildPhraseToSlug, glossary }            from './lib/glossary/glossary'
import { glossaryPlugin }                         from './lib/glossary/plugin'
import { bodyLinkPlugin }                         from './lib/markdown/body-link-plugin'
import { proseMarkPlugin }                        from './lib/markdown/prose-mark-plugin'
import { discoverRuleSlugs }                      from './lib/rules/discovery'
import { ruleLinkPlugin }                         from './lib/rules/link-plugin'
import { canonicalUrl }                           from './lib/shared/canonical-url'
import { REPO_URL, SHIKI_THEMES, SITE_HOSTNAME }  from './lib/shared/constants'
import { buildPageTimestamps }                    from './lib/shared/page-timestamps'
import { repoRoot, rulesDir }                     from './lib/shared/paths'
import { PRIMITIVES }                             from './lib/shared/registries'
import type { PrimitiveSlug }                     from './lib/shared/registries'
import { buildSidebar }                           from './lib/shared/sidebar'
import { toTitleCase }                            from './lib/shared/title-case'
import { TOOL_SEEDS }                             from './lib/shared/tools'
import { readCargoVersion }                       from './lib/shared/version'

const repoDir          = repoRoot(import.meta.url)
const version          = readCargoVersion(repoDir)
const pageTimestamps   = buildPageTimestamps(repoDir)
const discoveredRules  = discoverRuleSlugs(rulesDir(import.meta.url))
const validSlugs       = new Set(discoveredRules.map(r => r.slug))
const glossaryPhraseToSlug = buildPhraseToSlug(glossary)

export default defineConfig({
  cleanUrls     : true,
  description   : 'A Python typesetter for the reader.',
  head          : [
    ['link', { href: '/favicon.svg', rel: 'icon', type: 'image/svg+xml' }],
    ['meta', { content: '#dfbc97',                 name:     'theme-color'   }],
    ['meta', { content: 'summary_large_image',     name:     'twitter:card'  }],
    ['meta', { content: 'website',                 property: 'og:type'       }],
    ['meta', { content: 'Prose',                   property: 'og:site_name'  }]
  ],
  lastUpdated   : false,
  markdown      : {
    config      : md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
      md.use(ruleLinkPlugin(validSlugs))
      md.use(glossaryPlugin(glossaryPhraseToSlug))
      md.use(proseMarkPlugin)
      md.use(bodyLinkPlugin)
    },
    lineNumbers : false,
    theme       : SHIKI_THEMES
  },
  sitemap       : {
    hostname: SITE_HOSTNAME
  },
  themeConfig   : {
    editLink    : {
      pattern : `${REPO_URL}/edit/main/site/:path`,
      text    : 'Suggest an edit to this page'
    },
    logo        : { alt: 'prose', src: '/logo.svg' },
    nav         : [
      { activeMatch: '/guide/',        link: '/guide/installation', text: 'Guide'        },
      { activeMatch: '/reference/',    link: '/reference/cli',      text: 'Reference'    },
      { activeMatch: '/integrations/', link: '/integrations/ruff',  text: 'Integrations' },
      { activeMatch: '/rules/',        link: '/rules/',             text: 'Rules'        },
      { activeMatch: '/primitives/',   link: '/primitives/',        text: 'Primitives'   },
      {                                link: `${REPO_URL}/releases`, text: `v${version}` }
    ],
    outline     : { level: [2, 3] },
    search      : { provider: 'local' },
    sidebar     : buildSidebar(discoveredRules),
    siteTitle   : 'Prose',
    socialLinks : [
      { icon: 'github', link: REPO_URL }
    ]
  },
  title         : 'Prose',
  titleTemplate : ':title · Prose',
  transformHead({ pageData }) {
    const description = pageData.frontmatter.description ?? pageData.frontmatter.caption ?? 'A Python typesetter for the reader.'
    const title       = pageData.frontmatter.name ?? pageData.title ?? 'Prose'
    return [
      ['meta', { content: `${title} · Prose`, property: 'og:title'            }],
      ['meta', { content: description,        property: 'og:description'      }],
      ['meta', { content: `${title} · Prose`, name:     'twitter:title'       }],
      ['meta', { content: description,        name:     'twitter:description' }]
    ]
  },
  transformPageData(pageData) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { href: canonicalUrl(pageData.relativePath), rel: 'canonical' }
    ])
    const ts = pageTimestamps.get(pageData.relativePath)
    if (ts) pageData.lastUpdated = ts
    if (pageData.relativePath.startsWith('rules/') && !pageData.relativePath.endsWith('index.md')) {
      const slug = pageData.relativePath.replace(/^rules\/|\.md$/g, '')
      pageData.frontmatter.name ??= toTitleCase(slug, '-')
    }
    if (pageData.relativePath.startsWith('primitives/') && !pageData.relativePath.endsWith('index.md')) {
      const slug = pageData.relativePath.replace(/^primitives\/|\.md$/g, '')
      pageData.frontmatter.name ??= PRIMITIVES[slug as PrimitiveSlug]
    }
  },
  vite          : {
    css     : { postcss: { plugins: [postcssCustomMedia()] } },
    plugins : [groupIconVitePlugin({
      customIcon: {
        ...Object.fromEntries(Object.entries(TOOL_SEEDS).map(([slug, { icon }]) => [slug, icon])),
        gha: TOOL_SEEDS.github.icon
      }
    }) as never]
  }
})
