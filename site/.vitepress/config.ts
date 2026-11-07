import fs   from 'node:fs'
import path from 'node:path'

import tailwindcss                                from '@tailwindcss/vite'
import { defineConfig }                           from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'

import { discoverPrimitiveIndex, discoverPrimitives } from './lib/architecture/discovery'
import { checkBundleBudget }                          from './lib/config/bundle-budget'
import { canonicalUrl }                               from './lib/config/canonical-url'
import * as head                                      from './lib/config/head'
import { injectFixtures }                             from './lib/config/inject-fixtures'
import { ROBOTS_TXT }                                 from './lib/config/robots'
import { buildSidebar }                               from './lib/config/sidebar'
import * as stylesheetLayer                           from './lib/config/stylesheet-layer'
import { fixtureReloadPlugin }                        from './lib/fixtures/reload-plugin'
import { corpusLintFindings }                         from './lib/fixtures/walker'
import { glossary }                                   from './lib/glossary/entries'
import { glossaryHrefs }                              from './lib/glossary/hrefs'
import { buildPhraseToSlug }                          from './lib/glossary/phrase-map'
import { glossaryPlugin }                             from './lib/glossary/plugin'
import { readPlatforms }                              from './lib/guide/platforms'
import { bodyLinkPlugin }                             from './lib/markdown/body-link-plugin'
import { calloutPlugin }                              from './lib/markdown/callout-plugin'
import { codeOperatorPlugin }                         from './lib/markdown/code-operators'
import { emblemPlugin }                               from './lib/markdown/emblem-plugin'
import { HighlightCache }                             from './lib/markdown/highlight-cache'
import { lintDecorationTransformer }                  from './lib/markdown/lint-decorations'
import { proseMarkPlugin }                            from './lib/markdown/prose-mark-plugin'
import { scrollStopPlugin }                           from './lib/markdown/scroll-stop-plugin'
import { sectionHeadingPlugin }                       from './lib/markdown/section-heading-plugin'
import { TokenClasses }                               from './lib/markdown/token-classes'
import { wholeLineTokens }                            from './lib/markdown/whole-line-tokens'
import { compositionDir, readCompositionData }        from './lib/rules/composition'
import { discoverRuleIndex, discoverRules }           from './lib/rules/discovery'
import { assertCorpusIntegrity }                      from './lib/rules/integrity'
import { ruleLinkPlugin }                             from './lib/rules/link-plugin'
import { readRuleFixtures }                           from './lib/rules/rule-fixtures'
import { serveWasmPlugin }                            from './lib/sandbox/serve-plugin'
import * as constants                                 from './lib/shared/constants'
import { emblemsOf }                                  from './lib/shared/emblems'
import { PALETTE, paletteCss, themeCss }              from './lib/shared/palette'
import * as paths                                     from './lib/shared/paths'
import { SECTIONS }                                   from './lib/shared/registries'
import { sectionRoute }                               from './lib/shared/routes'
import { TOOL_SEEDS }                                 from './lib/shared/tools'
import * as version                                   from './lib/shared/version'

const crate                = paths.crateDir(import.meta.url)
const rulesDirectory       = paths.rulesDir(import.meta.url)
const siteDirectory        = paths.siteDir(import.meta.url)
const tailwindEntry        = path.join(siteDirectory, '.vitepress/theme/styles/tailwind.css')
const proseVersion         = version.readCargoVersion(crate)
const requiresPython       = version.readRequiresPython(crate)
const ruffRelease          = version.readRuffRelease(crate)
const ruleDiscovery        = discoverRules(rulesDirectory)
const ruleIndex            = discoverRuleIndex(rulesDirectory)
const discoveredRules      = ruleDiscovery.rules
const discoveredPrimitives = discoverPrimitives(paths.architectureDir(import.meta.url))
const emblems              = emblemsOf(readPlatforms(paths.repoRoot(import.meta.url)))
const primitiveIndex       = discoverPrimitiveIndex(paths.architectureDir(import.meta.url))
const glossaryPhraseToSlug = buildPhraseToSlug(glossary)
const themeColor           = PALETTE.ube
const fixturesRoot         = paths.fixturesDirFrom(crate)
const tokenClasses         = await TokenClasses.read()
const lintFindings         = corpusLintFindings(crate)
const codeTransformers     = [
  lintDecorationTransformer(lintFindings),
  wholeLineTokens,
  tokenClasses.transformer()
]
const highlightCache       = await HighlightCache.read(paths.highlightCacheDir(import.meta.url), {
  colorReplacements : constants.SHIKI_COLOR_REPLACEMENTS,
  data              : Object.fromEntries(lintFindings),
  theme             : constants.SHIKI_THEMES,
  transformers      : codeTransformers
})
const virtualSheets        = new Map([
  ['virtual:prose-palette.css', paletteCss],
  ['virtual:prose-tokens.css',  () => tokenClasses.css()]
])
const fixtureSets          = {
  composition  : readCompositionData(compositionDir(crate)),
  ruleFixtures : await readRuleFixtures(crate)
}

assertCorpusIntegrity(ruleDiscovery, discoveredPrimitives)

export default defineConfig({
  cacheDir      : paths.vitepressCacheDir(import.meta.url),
  cleanUrls     : true,
  description   : constants.SITE_TAGLINE,
  head: [
    ['link', { href: '/favicon.svg', rel: 'icon', type: 'image/svg+xml' }],
    ['meta', { content: themeColor,                name:     'theme-color'   }],
    ['meta', { content: 'summary_large_image',     name:     'twitter:card'  }],
    ['meta', { content: constants.SITE_NAME,       property: 'og:site_name'  }]
  ],
  lastUpdated   : true,
  markdown: {
    codeTransformers,
    config: md => {
      md.use(highlightCache.plugin)
      md.use(groupIconMdPlugin)
      md.use(ruleLinkPlugin(ruleIndex, primitiveIndex))
      md.use(glossaryPlugin(glossaryPhraseToSlug, glossaryHrefs(glossary, ruleIndex)))
      md.use(sectionHeadingPlugin)
      md.use(calloutPlugin)
      md.use(emblemPlugin(emblems))
      md.use(bodyLinkPlugin)
      md.use(proseMarkPlugin)
      md.use(codeOperatorPlugin)
      md.use(scrollStopPlugin)
    },
    colorReplacements : constants.SHIKI_COLOR_REPLACEMENTS,
    lineNumbers       : false,
    theme             : constants.SHIKI_THEMES
  },
  sitemap: { hostname: constants.SITE_HOSTNAME },
  themeConfig: {
    editLink: {
      pattern : `${constants.REPO_URL}/edit/main/site/:path`,
      text    : 'Suggest an edit to this page'
    },
    logo      : { alt: '', src: '/logo.svg' },
    nav: [
      ...SECTIONS.map(({ label, slug }) =>
        ({ activeMatch: sectionRoute(slug), link: sectionRoute(slug), text: label })),
      { link: `${constants.REPO_URL}/releases`, text: `v${proseVersion}` }
    ],
    outline   : { level: [2, 3] },
    search    : { provider: 'local' },
    sidebar   : buildSidebar(discoveredPrimitives, discoveredRules, siteDirectory),
    siteTitle : constants.SITE_NAME,
    socialLinks: [
      { icon: 'github', link: constants.REPO_URL }
    ]
  },
  title         : constants.SITE_NAME,
  titleTemplate : head.TITLE_TEMPLATE,
  async buildEnd(siteConfig) {
    fs.writeFileSync(path.join(siteConfig.outDir, 'robots.txt'), ROBOTS_TXT)
    stylesheetLayer.layerIconStylesheet(path.join(siteConfig.outDir, siteConfig.assetsDir))
    const { buildOgCards } = await import('./lib/og/render/build')
    await buildOgCards(siteConfig.srcDir, siteConfig.pages, siteConfig.outDir)
    checkBundleBudget(siteConfig.outDir)
    await highlightCache.flush()
  },
  transformHead({ pageData }) {
    return head.pageHead(pageData, proseVersion)
  },
  async transformPageData(pageData, { siteConfig }) {
    pageData.frontmatter ||= {}
    pageData.frontmatter.proseVersion   = proseVersion
    pageData.frontmatter.requiresPython = requiresPython
    pageData.frontmatter.ruffVersion    = ruffRelease
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push([
      'link',
      { href: canonicalUrl(pageData.relativePath), rel: 'canonical' }
    ])
    head.describePage(pageData)
    if (pageData.relativePath.startsWith('rules/')) {
      pageData.frontmatter.name ??= ruleIndex.get(path.basename(pageData.relativePath, '.md'))?.name
    }
    await injectFixtures(pageData, crate, fixtureSets, siteConfig.srcDir)
  },
  vite: {
    build: { chunkSizeWarningLimit: 5000 },
    oxc: { jsx: constants.VUE_JSX },
    plugins: [{
      enforce   : 'pre',
      load      : id => virtualSheets.get(id.replace(/^\0/, ''))?.(),
      name      : 'prose-sheets',
      resolveId : id => virtualSheets.has(id) ? `\0${id}` : undefined,
      transform : (code, id) => id.split('?')[0] === tailwindEntry ? code + themeCss() : undefined
    },
    fixtureReloadPlugin(fixturesRoot),
    serveWasmPlugin(path.join(siteDirectory, 'public')),
    stylesheetLayer.stylesheetLayerPlugin(tailwindEntry),
    tailwindcss(),
    groupIconVitePlugin({
      customIcon: {
        ...Object.fromEntries(Object.entries(TOOL_SEEDS).map(([slug, { icon }]) => [slug, icon])),
        gha: TOOL_SEEDS.github.icon
      }
    }) as never]
  }
})
