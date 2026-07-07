import fs   from 'node:fs'
import path from 'node:path'

import postcssGlobalData                          from '@csstools/postcss-global-data'
import postcssCustomMedia                         from 'postcss-custom-media'
import githubDark                                 from 'shiki/themes/github-dark.mjs'
import { defineConfig }                           from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'
import { tabsMarkdownPlugin }                     from 'vitepress-plugin-tabs'

import { canonicalUrl }                               from './lib/config/canonical-url'
import { pageHead }                                   from './lib/config/head'
import { ROBOTS_TXT }                                 from './lib/config/robots'
import { buildSidebar }                               from './lib/config/sidebar'
import { corpusLintFindings }                         from './lib/fixtures/walker'
import { glossary }                                   from './lib/glossary/entries'
import { glossaryHrefs }                              from './lib/glossary/hrefs'
import { buildPhraseToSlug }                          from './lib/glossary/phrase-map'
import { glossaryPlugin }                             from './lib/glossary/plugin'
import { bodyLinkPlugin }                             from './lib/markdown/body-link-plugin'
import { lintDecorationTransformer }                  from './lib/markdown/lint-decorations'
import { proseMarkPlugin }                            from './lib/markdown/prose-mark-plugin'
import { discoverPrimitiveIndex, discoverPrimitives } from './lib/primitives/discovery'
import { discoverRuleIndex, discoverRules }           from './lib/rules/discovery'
import { assertCorpusIntegrity }                      from './lib/rules/integrity'
import { ruleLinkPlugin }                             from './lib/rules/link-plugin'
import * as constants                                 from './lib/shared/constants'
import { PALETTE, paletteCss }                        from './lib/shared/palette'
import * as paths                                     from './lib/shared/paths'
import { SECTIONS }                                   from './lib/shared/registries'
import { sectionRoute }                               from './lib/shared/routes'
import { toTitleCase }                                from './lib/shared/title-case'
import { TOOL_SEEDS }                                 from './lib/shared/tools'
import { readCargoVersion }                           from './lib/shared/version'

const crate                = paths.crateDir(import.meta.url)
const rulesDirectory       = paths.rulesDir(import.meta.url)
const version              = readCargoVersion(crate)
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
  cacheDir      : paths.vitepressCacheDir(import.meta.url),
  cleanUrls     : true,
  description   : constants.SITE_TAGLINE,
  head: [
    ['link', { href: '/favicon.svg', rel: 'icon', type: 'image/svg+xml' }],
    ['meta', { content: themeColor,                name:     'theme-color'   }],
    ['meta', { content: 'summary_large_image',     name:     'twitter:card'  }],
    ['meta', { content: 'Prose',                   property: 'og:site_name'  }],
    ['style', {}, `:root{--prose-shiki-dark-bg:${shikiDarkBg}}`]
  ],
  lastUpdated   : true,
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
  sitemap: { hostname: constants.SITE_HOSTNAME },
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
    if (pageData.relativePath.startsWith('rules/') && !pageData.relativePath.endsWith('index.md')) {
      pageData.frontmatter.name ??= toTitleCase(path.basename(pageData.relativePath, '.md'), '-')
    }
    if (pageData.relativePath.startsWith('primitives/') && !pageData.relativePath.endsWith('index.md')) {
      const slug = path.basename(pageData.relativePath, '.md')
      pageData.frontmatter.name ??= primitiveIndex.get(slug)?.name
    }
  },
  vite: {
    build: { chunkSizeWarningLimit: 4000 },
    css: {
      postcss: {
        plugins: [
          postcssGlobalData({
            files: [path.join(paths.siteDir(import.meta.url), '.vitepress/theme/styles/tokens.css')]
          }),
          postcssCustomMedia()
        ]
      }
    },
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
