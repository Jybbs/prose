import postcssCustomMedia                         from 'postcss-custom-media'
import githubDark                                 from 'shiki/themes/github-dark.mjs'
import { defineConfig }                           from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin }                     from 'vitepress-plugin-tabs'

import { buildPhraseToSlug }                 from './lib/glossary/glossary'
import { glossary }                          from './lib/glossary/entries'
import { glossaryPlugin }                    from './lib/glossary/plugin'
import { bodyLinkPlugin }                    from './lib/markdown/body-link-plugin'
import { lintDecorationTransformer }         from './lib/markdown/lint-decorations'
import { proseMarkPlugin }                   from './lib/markdown/prose-mark-plugin'
import { discoverPrimitives }                from './lib/primitives/discovery'
import { discoverRuleSlugs }                 from './lib/rules/discovery'
import { ruleLinkPlugin }                    from './lib/rules/link-plugin'
import { canonicalUrl }                      from './lib/config/canonical-url'
import { ogImageUrl }                        from './lib/config/og-url'
import { resolveToken }                      from './lib/og/colors'
import { CARD_HEIGHT, CARD_WIDTH }           from './lib/og/parts'
import { REPO_URL, SHIKI_THEMES, SITE_HOSTNAME, SITE_TAGLINE } from './lib/shared/constants'
import { buildPageTimestamps }               from './lib/config/page-timestamps'
import { primitivesDir, repoRoot, rulesDir } from './lib/shared/paths'
import { buildSidebar }                      from './lib/config/sidebar'
import { toTitleCase }                       from './lib/shared/title-case'
import { TOOL_SEEDS }                        from './lib/shared/tools'
import { readCargoVersion }                  from './lib/shared/version'

const repoDir              = repoRoot(import.meta.url)
const version              = readCargoVersion(repoDir)
const pageTimestamps       = buildPageTimestamps(repoDir)
const discoveredRules      = discoverRuleSlugs(rulesDir(import.meta.url))
const discoveredPrimitives = discoverPrimitives(primitivesDir(import.meta.url))
const primitiveNames       = new Map(discoveredPrimitives.map(p => [p.slug as string, p.name]))
const validSlugs           = new Set(discoveredRules.map(r => r.slug))
const glossaryPhraseToSlug = buildPhraseToSlug(glossary)
const shikiDarkBg          = githubDark.colors?.['editor.background'] as string
const themeColor           = resolveToken('prose-c-ube')

export default defineConfig({
  cacheDir      : `${repoDir}/.cache/vitepress`,
  cleanUrls     : true,
  description   : SITE_TAGLINE,
  head          : [
    ['link', { href: '/favicon.svg', rel: 'icon', type: 'image/svg+xml' }],
    ['meta', { content: themeColor,                name:     'theme-color'   }],
    ['meta', { content: 'summary_large_image',     name:     'twitter:card'  }],
    ['meta', { content: 'website',                 property: 'og:type'       }],
    ['meta', { content: 'Prose',                   property: 'og:site_name'  }],
    ['style', {}, `:root{--prose-shiki-dark-bg:${shikiDarkBg}}`]
  ],
  lastUpdated   : false,
  markdown      : {
    codeTransformers : [lintDecorationTransformer],
    config      : md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
      md.use(ruleLinkPlugin(validSlugs, primitiveNames))
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
      { activeMatch: '/usage/',        link: '/usage/',             text: 'Usage'        },
      { activeMatch: '/reference/',    link: '/reference/',         text: 'Reference'    },
      { activeMatch: '/integrations/', link: '/integrations/',      text: 'Integrations' },
      { activeMatch: '/rules/',        link: '/rules/',             text: 'Rules'        },
      { activeMatch: '/primitives/',   link: '/primitives/',        text: 'Primitives'   },
      {                                link: `${REPO_URL}/releases`, text: `v${version}` }
    ],
    outline     : { level: [2, 3] },
    search      : { provider: 'local' },
    sidebar     : buildSidebar(discoveredRules, discoveredPrimitives),
    siteTitle   : 'Prose',
    socialLinks : [
      { icon: 'github', link: REPO_URL }
    ]
  },
  title         : 'Prose',
  titleTemplate : ':title · Prose',
  async buildEnd(siteConfig) {
    const { buildOgCards } = await import('./lib/og/build')
    await buildOgCards(siteConfig.srcDir, siteConfig.pages, siteConfig.outDir)
  },
  transformHead({ pageData }) {
    const isLanding   = pageData.relativePath === 'index.md'
    const description = pageData.frontmatter.description ?? pageData.frontmatter.caption ?? SITE_TAGLINE
    const title       = pageData.frontmatter.name ?? pageData.title ?? 'Prose'
    const ogImage     = ogImageUrl(pageData.relativePath)
    const ogTitle     = isLanding ? 'Prose'                                       : `${title} · Prose`
    const ogAlt       = isLanding ? 'Prose, a Python typesetter for the reader.'  : `${title} card`
    const ogUrl       = canonicalUrl(pageData.relativePath)
    return [
      ['meta', { content: ogTitle,             property: 'og:title'           }],
      ['meta', { content: description,         property: 'og:description'     }],
      ['meta', { content: ogUrl,               property: 'og:url'             }],
      ['meta', { content: 'en_US',             property: 'og:locale'          }],
      ['meta', { content: ogImage,             property: 'og:image'           }],
      ['meta', { content: String(CARD_WIDTH),  property: 'og:image:width'     }],
      ['meta', { content: String(CARD_HEIGHT), property: 'og:image:height'    }],
      ['meta', { content: 'image/png',         property: 'og:image:type'      }],
      ['meta', { content: ogAlt,               property: 'og:image:alt'       }],
      ['meta', { content: ogImage,             name:     'twitter:image'      }],
      ['meta', { content: ogAlt,               name:     'twitter:image:alt'  }]
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
      pageData.frontmatter.name ??= primitiveNames.get(slug)
    }
  },
  vite          : {
    build   : {
      chunkSizeWarningLimit : 4000,
      rollupOptions         : {
        output: {
          manualChunks(id) {
            if (id.includes('/shiki-magic-move/')) return 'shiki-magic-move'
            if (id.includes('/floating-vue/'))     return 'floating-vue'
            if (id.includes('/@vueuse/'))          return 'vueuse'
          }
        }
      }
    },
    css     : { postcss: { plugins: [postcssCustomMedia()] } },
    plugins : [groupIconVitePlugin({
      customIcon: {
        ...Object.fromEntries(Object.entries(TOOL_SEEDS).map(([slug, { icon }]) => [slug, icon])),
        gha: TOOL_SEEDS.github.icon
      }
    }) as never]
  }
})
