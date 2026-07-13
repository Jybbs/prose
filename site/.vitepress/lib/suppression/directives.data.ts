import { defineLoader } from 'vitepress'

import { getRenderer, inlineNodeField }       from '../markdown/renderer'
import type { InlineNode }                    from '../markdown/inline-nodes'
import { type DirectivePart, directiveParts } from './directive-parts'
import type { ScopeKey }                      from './scopes'

export interface Directive {
  effectNodes : InlineNode[]
  example     : string
  form        : string
  id          : string
  pairId     ?: string
  pairRole   ?: 'closes' | 'opens'
  parts       : DirectivePart[]
  scope       : ScopeKey
}

interface DirectiveSource {
  effect    : string
  example   : string
  form      : string
  id        : string
  pairId   ?: string
  pairRole ?: 'closes' | 'opens'
  scope     : ScopeKey
}

const SOURCES: readonly DirectiveSource[] = [
  {
    effect  : 'Suppresses every *Prose* rewrite for the entire file. Declared on a comment '
            + 'line near the top.',
    example : '# prose: off\n\ndef messy(): pass',
    form    : '# prose: off',
    id      : 'prose-off',
    scope   : 'file'
  },
  {
    effect   : 'Opens a region every auto-fix rule leaves untouched, so a hand-tuned block '
             + 'survives the formatter pass intact.',
    example  : '# fmt: off\nkeep_this_block_exactly_as_written = (1,2,3)\n# fmt: on',
    form     : '# fmt: off',
    id       : 'fmt-off',
    pairId   : 'fmt-on',
    pairRole : 'opens',
    scope    : 'block'
  },
  {
    effect   : 'Closes the suppressed region. Formatting resumes on the following line.',
    example  : '# fmt: off\nkeep_this_block_exactly_as_written = (1,2,3)\n# fmt: on',
    form     : '# fmt: on',
    id       : 'fmt-on',
    pairId   : 'fmt-off',
    pairRole : 'closes',
    scope    : 'block'
  },
  {
    effect   : 'Alias for `# fmt: off`. Recognized to ease migration from yapf.',
    example  : '# yapf: disable\nkeep_this_block_exactly_as_written = (1,2,3)\n# yapf: enable',
    form     : '# yapf: disable',
    id       : 'yapf-disable',
    pairId   : 'yapf-enable',
    pairRole : 'opens',
    scope    : 'block'
  },
  {
    effect   : 'Alias for `# fmt: on`. Closes a yapf-style suppressed region.',
    example  : '# yapf: disable\nkeep_this_block_exactly_as_written = (1,2,3)\n# yapf: enable',
    form     : '# yapf: enable',
    id       : 'yapf-enable',
    pairId   : 'yapf-disable',
    pairRole : 'closes',
    scope    : 'block'
  },
  {
    effect  : 'Every ordering rule leaves the dict entries in their authored order. Scopes '
            + 'to that one dict literal.',
    example : 'config = {  # prose: keep\n    "stage_one"   : True,\n    "stage_two"   : '
            + 'False,\n}',
    form    : '# prose: keep',
    id      : 'prose-keep',
    scope   : 'dict'
  },
  {
    effect  : 'Every auto-fix rule skips the line carrying the directive. Pairs with '
            + '`[<rule>, ...]` to narrow the scope.',
    example : 'data = {"a": 1, "b": 2, "c": 3}  # fmt: skip',
    form    : '# fmt: skip',
    id      : 'fmt-skip',
    scope   : 'line'
  },
  {
    effect  : 'Alias for `# fmt: skip`. Every auto-fix rule skips the line carrying the directive.',
    example : 'data = {"a": 1, "b": 2, "c": 3}  # prose: skip',
    form    : '# prose: skip',
    id      : 'prose-skip',
    scope   : 'line'
  },
  {
    effect  : 'Only the listed auto-fix rules skip the line. Two bracketed directives on one '
            + 'line union their rule slugs.',
    example : 'foo = 1  # prose: skip[align-equals, strip-trailing-commas]',
    form    : '# prose: skip[<rule>, ...]',
    id      : 'prose-skip-rules',
    scope   : 'line'
  },
  {
    effect  : 'Every lint rule skips the line. Pairs with `[<rule>, ...]` to narrow the scope.',
    example : 'helper = build_helper()  # prose: ignore',
    form    : '# prose: ignore',
    id      : 'prose-ignore',
    scope   : 'line'
  },
  {
    effect  : 'Only the listed lint rules skip the line. Two bracketed directives on one '
            + 'line union their rule slugs.',
    example : 'TIMEOUT = 30  # prose: ignore[reassigned-constants, single-use-variables]',
    form    : '# prose: ignore[<rule>, ...]',
    id      : 'prose-ignore-rules',
    scope   : 'line'
  }
]

declare const data: readonly Directive[]
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<readonly Directive[]> {
    const md = await getRenderer()
    return inlineNodeField(md, SOURCES, 'effect')
      .map(source => ({ ...source, parts: directiveParts(source.form) }))
  }
})
