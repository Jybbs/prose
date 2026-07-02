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

// Wiki-links and glossary terms resolve first, then the word-mark, then
// body-link last so it reaches the anchors the earlier plugins emit.
export const proseProcessor = unified({
  remarkPlugins: [[remarkRuleLinks, vocab], [remarkGlossary, vocab], remarkProseMark, remarkBodyLink]
})

export const shikiConfig: ShikiConfig = { themes: SHIKI_THEMES }

export const lintFlagPlugin = pluginLintFlag(discoverLintFindings(siteRoot))
