import { unified }          from '@astrojs/markdown-remark'
import type { ShikiConfig } from '@astrojs/markdown-remark'

import { discoverDocsVocab }    from '../content/docs-vocab'
import { discoverLintFindings } from '../content/lint-findings'
import { SHIKI_THEMES }         from '../shared/constants'
import { remarkBodyLink }       from './body-link'
import { remarkGlossary }       from './glossary-linker'
import { pluginLintFlag }       from './lint-flag'
import { remarkProseMark }      from './prose-mark'
import { remarkRuleLinks }      from './rule-links'

// The docs vocabulary and lint findings read the filesystem once, at config
// load, since the page-body plugins bind them before the content collections
// exist.
const siteRoot = new URL('../../../', import.meta.url)
const vocab    = discoverDocsVocab(siteRoot)

// The default remark processor carrying the page-body plugin order, set as
// `markdown.processor` and shared with the standalone render path. Wiki-links
// and glossary terms resolve first, then the word-mark, then body-link last so
// it reaches the anchors the earlier plugins emit.
export const proseProcessor = unified({
  remarkPlugins: [[remarkRuleLinks, vocab], [remarkGlossary, vocab], remarkProseMark, remarkBodyLink]
})

// Shiki stays cross-cutting, so `markdown.shikiConfig` and the standalone
// renderer both take it and highlight with the one dual-theme set.
export const shikiConfig: ShikiConfig = { themes: SHIKI_THEMES }

// The lint-decoration plugin, bound to the findings a `lint=` fence names, for
// Starlight's Expressive Code integration.
export const lintFlagPlugin = pluginLintFlag(discoverLintFindings(siteRoot))
