import fs   from 'node:fs'
import path from 'node:path'

import postcssCustomMedia                         from 'postcss-custom-media'
import githubDark                                 from 'shiki/themes/github-dark.mjs'
import { defineConfig }                           from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin }                     from 'vitepress-plugin-tabs'

import { corpusLintFindings }               from './lib/fixtures/walker'
import { buildPhraseToSlug }                from './lib/glossary/phrase-map'
import { glossary }                         from './lib/glossary/entries'
import { glossaryHrefs }                    from './lib/glossary/hrefs'
import { glossaryPlugin }                   from './lib/glossary/plugin'
import { bodyLinkPlugin }                   from './lib/markdown/body-link-plugin'
import { lintDecorationTransformer }        from './lib/markdown/lint-decorations'
import { proseMarkPlugin }                  from './lib/markdown/prose-mark-plugin'
import { discoverPrimitiveIndex, discoverPrimitives } from './lib/primitives/discovery'
import { discoverRuleIndex, discoverRules } from './lib/rules/discovery'
import { assertCorpusIntegrity }            from './lib/rules/integrity'
import { ruleLinkPlugin }                   from './lib/rules/link-plugin'
import { canonicalUrl }                     from './lib/config/canonical-url'
import { pageHead }                         from './lib/config/head'
import { ROBOTS_TXT }                       from './lib/config/robots'
import { attachLastmod }                    from './lib/config/sitemap'
import { PALETTE, paletteCss }              from './lib/shared/palette'
import * as constants                       from './lib/shared/constants'
import { buildPageTimestamps }              from './lib/config/page-timestamps'
import * as paths                           from './lib/shared/paths'
import { SECTIONS }                         from './lib/shared/registries'
import { sectionRoute }                     from './lib/shared/routes'
import { buildSidebar }                     from './lib/config/sidebar'
import { toTitleCase }                      from './lib/shared/title-case'
import { TOOL_SEEDS }                       from './lib/shared/tools'
import { readCargoVersion }                 from './lib/shared/version'

const repoDir              = paths.repoRoot(import.meta.url)
const crate                = paths.crateDir(import.meta.url)
const rulesDirectory       = paths.rulesDir(import.meta.url)
const version              = readCargoVersion(crate)
const pageTimestamps       = buildPageTimestamps(repoDir)
const ruleDiscovery        = discoverRules(rulesDirectory)
const ruleIndex            = discoverRuleIndex(rulesDirectory)
const discoveredRules      = ruleDiscovery.rules
const discoveredPrimitives = discoverPrimitives(paths.primitivesDir(import.meta.url))
const primitiveIndex       = discoverPrimitiveIndex(paths.primitivesDir(import.meta.url))
const glossaryPhraseToSlug = buildPhraseToSlug(glossary)
const shikiDarkBg          = githubDark.colors?.['editor.background'] as string
const themeColor           = PALETTE.ube

assertCorpusIntegrity(ruleDiscovery, discoveredPrimitives)

export default defineConfig({
  cacheDir      : `${repoDir}/.cache/vitepress`,
  cleanUrls     : true,
  description   : constants.SITE_TAGLINE,
  head: [
    ['link', { href: '/favicon.svg', rel: 'icon', type: 'image/svg+xml' }],
    ['meta', { content: themeColor,                name:     'theme-color'   }],
    ['meta', { content: 'summary_large_image',     name:     'twitter:card'  }],
    ['meta', { content: 'Prose',                   property: 'og:site_name'  }],
    ['style', {}, `:root{--prose-shiki-dark-bg:${shikiDarkBg}}`]
  ],
  lastUpdated   : false,
  markdown: {
    codeTransformers : [lintDecorationTransformer(corpusLintFindings(crate))],
    config: md => {
      md.use(groupIconMdPlugin)
      md.use(tabsMarkdownPlugin)
      md.use(ruleLinkPlugin(ruleIndex, primitiveIndex))
      md.use(glossaryPlugin(glossaryPhraseToSlug, glossaryHrefs(glossary, ruleIndex)))
      md.use(proseMarkPlugin)
      md.use(bodyLinkPlugin)
    },
    lineNumbers : false,
    theme       : constants.SHIKI_THEMES
  },
  sitemap: {
    hostname       : constants.SITE_HOSTNAME,
    transformItems : items => attachLastmod(items, pageTimestamps)
  },
  themeConfig: {
    editLink: {
      pattern : `${constants.REPO_URL}/edit/main/site/:path`,
      text    : 'Suggest an edit to this page'
    },
    logo      : { alt: 'prose', src: '/logo.svg' },
    nav: [
      ...SECTIONS.map(({ label, slug }) =>
        ({ activeMatch: sectionRoute(slug), link: sectionRoute(slug), text: label })),
      { link: `${constants.REPO_URL}/releases`, text: `v${version}` }
    ],
    outline   : { level: [2, 3] },
    search    : { provider: 'local' },
    sidebar   : buildSidebar(discoveredPrimitives, discoveredRules, paths.siteDir(import.meta.url)),
    siteTitle : 'Prose',
    socialLinks: [
      { icon: 'github', link: constants.REPO_URL }
    ]
  },
  title         : 'Prose',
  titleTemplate : ':title · Prose',
  async buildEnd(siteConfig) {
    fs.writeFileSync(path.join(siteConfig.outDir, 'robots.txt'), ROBOTS_TXT)
    const { buildOgCards } = await import('./lib/og/render/build')
    await buildOgCards(siteConfig.srcDir, siteConfig.pages, siteConfig.outDir)
  },
  transformHead({ pageData }) {
    return pageHead(pageData, version)
  },
  transformPageData(pageData) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { href: canonicalUrl(pageData.relativePath), rel: 'canonical' }
    ])
    if (!pageData.description && typeof pageData.frontmatter.caption === 'string') {
      pageData.description = pageData.frontmatter.caption
    }
    const ts = pageTimestamps.get(pageData.relativePath)
    if (ts) pageData.lastUpdated = ts
    if (pageData.relativePath.startsWith('rules/') && !pageData.relativePath.endsWith('index.md')) {
      pageData.frontmatter.name ??= toTitleCase(path.basename(pageData.relativePath, '.md'), '-')
    }
    if (pageData.relativePath.startsWith('primitives/') && !pageData.relativePath.endsWith('index.md')) {
      const slug = pageData.relativePath.replace(/^primitives\/|\.md$/g, '')
      pageData.frontmatter.name ??= primitiveIndex.get(slug)?.name
    }
  },
  vite: {
    build: {
      chunkSizeWarningLimit: 4000,
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (id.includes('/shiki-magic-move/')) return 'shiki-magic-move'
            if (id.includes('/floating-vue/'))     return 'floating-vue'
            if (id.includes('/@vueuse/'))          return 'vueuse'
          }
        }
      }
    },
    css: { postcss: { plugins: [postcssCustomMedia()] } },
    plugins: [{
      load      : id => id === '\0virtual:prose-palette.css' ? paletteCss() : undefined,
      name      : 'prose-palette',
      resolveId : id => id === 'virtual:prose-palette.css' ? '\0virtual:prose-palette.css' : undefined
    }, groupIconVitePlugin({
      customIcon: {
        ...Object.fromEntries(Object.entries(TOOL_SEEDS).map(([slug, { icon }]) => [slug, icon])),
        gha: TOOL_SEEDS.github.icon
      }
    }) as never]
  }
})
