import postcssCustomMedia                         from 'postcss-custom-media'
import { defineConfig }                            from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin }                     from 'vitepress-plugin-tabs'

import { buildPhraseToSlug, glossary }            from './lib/glossary/glossary'
import { glossaryPlugin }                         from './lib/glossary/plugin'
import { bodyLinkPlugin }                         from './lib/markdown/body-link-plugin'
import { discoverRuleSlugs }                      from './lib/rules/discovery'
import { ruleLinkPlugin }                         from './lib/rules/link-plugin'
import { REPO_URL, SHIKI_THEMES, SITE_HOSTNAME }  from './lib/shared/constants'
import { buildPageTimestamps }                    from './lib/shared/page-timestamps'
import { repoRoot, rulesDir }                     from './lib/shared/paths'
import { buildSidebar }                           from './lib/shared/sidebar'
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
    ['link', { href: '/favicon.svg', rel: 'icon', type: 'image/svg+xml' }]
  ],
  lastUpdated   : false,
  markdown      : {
    config      : md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
      md.use(ruleLinkPlugin(validSlugs))
      md.use(glossaryPlugin(glossaryPhraseToSlug))
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
  transformPageData(pageData) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { href: `${SITE_HOSTNAME}/${pageData.relativePath.replace(/(^|\/)index\.md$/, '$1').replace(/\.md$/, '')}`, rel: 'canonical' }
    ])
    const ts = pageTimestamps.get(pageData.relativePath)
    if (ts) pageData.lastUpdated = ts
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
