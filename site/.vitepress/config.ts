import fs   from 'node:fs'
import path from 'node:path'

import tailwindcss                                from '@tailwindcss/vite'
import { defineConfig, type MarkdownOptions }     from 'vitepress'
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'

import { discoverPrimitiveIndex, discoverPrimitives } from './lib/architecture/discovery.ts'
import { checkBundleBudget }                          from './lib/config/bundle-budget.ts'
import { canonicalUrl }                               from './lib/config/canonical-url.ts'
import * as head                                      from './lib/config/head.ts'
import { injectFixtures }                             from './lib/config/inject-fixtures.ts'
import { ROBOTS_TXT }                                 from './lib/config/robots.ts'
import { buildSidebar }                               from './lib/config/sidebar.ts'
import * as stylesheetLayer                           from './lib/config/stylesheet-layer.ts'
import { fixtureReloadPlugin }                        from './lib/fixtures/reload-plugin.ts'
import { corpusLintFindings }                         from './lib/fixtures/walker.ts'
import { glossary }                                   from './lib/glossary/entries.ts'
import { glossaryHrefs }                              from './lib/glossary/hrefs.ts'
import { buildPhraseToSlug }                          from './lib/glossary/phrase-map.ts'
import { glossaryPlugin }                             from './lib/glossary/plugin.ts'
import { readPlatforms }                              from './lib/guide/platforms.ts'
import { bodyLinkPlugin }                             from './lib/markdown/body-link-plugin.ts'
import { calloutPlugin }                              from './lib/markdown/callout-plugin.ts'
import { codeOperatorPlugin }                         from './lib/markdown/code-operators.ts'
import { codeTransformers }                           from './lib/markdown/code-transformers.ts'
import { configFormsPlugin }                          from './lib/markdown/config-forms-plugin.ts'
import { emblemPlugin }                               from './lib/markdown/emblem-plugin.ts'
import { HighlightCache }                             from './lib/markdown/highlight-cache.ts'
import { proseMarkPlugin }                            from './lib/markdown/prose-mark-plugin.ts'
import { scrollStopPlugin }                           from './lib/markdown/scroll-stop-plugin.ts'
import { sectionHeadingPlugin }                       from './lib/markdown/section-heading-plugin.ts'
import { TokenClasses }                               from './lib/markdown/token-classes.ts'
import { compositionDir, readCompositionData }        from './lib/rules/composition.ts'
import { discoverRuleIndex, discoverRules }           from './lib/rules/discovery.ts'
import { assertCorpusIntegrity }                      from './lib/rules/integrity.ts'
import { ruleLinkPlugin }                             from './lib/rules/link-plugin.ts'
import { readRuleFixtures }                           from './lib/rules/rule-fixtures.ts'
import { serveWasmPlugin }                            from './lib/sandbox/serve-plugin.ts'
import * as constants                                 from './lib/shared/constants.ts'
import { emblemsOf }                                  from './lib/shared/emblems.ts'
import { PALETTE, paletteCss, themeCss }              from './lib/shared/palette.ts'
import * as paths                                     from './lib/shared/paths.ts'
import { readPublishedVersion }                       from './lib/shared/published-version.ts'
import { SECTIONS }                                   from './lib/shared/registries.ts'
import { readRenderGraph }                            from './lib/shared/render-graph.ts'
import { sectionRoute }                               from './lib/shared/routes.ts'
import { TOOL_SEEDS }                                 from './lib/shared/tools.ts'
import * as version                                   from './lib/shared/version.ts'

const crate                = paths.crateDir(import.meta.url)
const rulesDirectory       = paths.rulesDir(import.meta.url)
const siteDirectory        = paths.siteDir(import.meta.url)
const pagesDirectory       = paths.pagesDir(import.meta.url)
const publicDirectory      = paths.publicDir(import.meta.url)
const tailwindEntry        = path.join(siteDirectory, '.vitepress/theme/styles/tailwind.css')
const proseVersion         = await readPublishedVersion(crate, paths.fetchCacheDir(import.meta.url))
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
const highlightOptions     = {
  codeTransformers  : codeTransformers(lintFindings, tokenClasses),
  colorReplacements : constants.SHIKI_COLOR_REPLACEMENTS,
  theme             : constants.SHIKI_THEMES
} satisfies MarkdownOptions
const highlightCache       = await HighlightCache.read(paths.highlightCacheDir(import.meta.url), {
  data     : Object.fromEntries(lintFindings),
  graph    : await readRenderGraph(path.join(siteDirectory, '.vitepress/lib/markdown')),
  lockfile : fs.readFileSync(path.join(paths.repoRoot(import.meta.url), 'bun.lock'), 'utf8'),
  options  : highlightOptions
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
    ...highlightOptions,
    config: md => {
      md.use(highlightCache.plugin)
      md.use(configFormsPlugin)
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
    lineNumbers: false
  },
  sitemap: { hostname: constants.SITE_HOSTNAME },
  srcDir        : pagesDirectory,
  themeConfig: {
    editLink: {
      pattern : `${constants.REPO_URL}/edit/main/site/pages/:path`,
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
    sidebar   : buildSidebar(discoveredPrimitives, discoveredRules, pagesDirectory),
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
    const { buildOgCards } = await import('./lib/og/render/build.ts')
    await buildOgCards(siteConfig.srcDir, siteConfig.pages, siteConfig.outDir, proseVersion)
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
    serveWasmPlugin(publicDirectory),
    stylesheetLayer.stylesheetLayerPlugin(tailwindEntry),
    tailwindcss(),
    groupIconVitePlugin({
      customIcon: {
        ...Object.fromEntries(Object.entries(TOOL_SEEDS).map(([slug, { icon }]) => [slug, icon])),
        gha: TOOL_SEEDS.github.icon
      }
    }) as never],
    publicDir: publicDirectory
  }
})
