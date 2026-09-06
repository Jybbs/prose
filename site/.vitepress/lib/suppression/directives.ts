import type { ScopeKey } from './scopes'

export interface DirectiveSource {
  blurb     : string
  effect    : string
  example   : string
  form      : string
  id        : string
  pairId   ?: string
  pairRole ?: 'closes' | 'opens'
  scope     : ScopeKey
}

export const DIRECTIVES: readonly DirectiveSource[] = [
  {
    blurb   : 'File-level suppression, so every rule skips the file.',
    effect  : 'Opens a region every rewrite and every lint skips, running from its own line to '
            + 'a `# prose: on` or the end of the file. On a comment line of its own near the '
            + 'top, it exempts the whole file.',
    example : '# prose: off\n\ndef messy(): pass',
    form    : '# prose: off',
    id      : 'prose-off',
    scope   : 'file'
  },
  {
    blurb    : 'Opens a block no rule touches.',
    effect   : 'Opens a region every rule leaves as written, rewrites and lints alike, so a '
             + 'hand-laid block survives the run.',
    example  : '# fmt: off\nkeep_this_block_exactly_as_written = (1,2,3)\n# fmt: on',
    form     : '# fmt: off',
    id       : 'fmt-off',
    pairId   : 'fmt-on',
    pairRole : 'opens',
    scope    : 'block'
  },
  {
    blurb    : 'Closes the block.',
    effect   : 'Closes the exempt region. Formatting resumes on the next line.',
    example  : '# fmt: off\nkeep_this_block_exactly_as_written = (1,2,3)\n# fmt: on',
    form     : '# fmt: on',
    id       : 'fmt-on',
    pairId   : 'fmt-off',
    pairRole : 'closes',
    scope    : 'block'
  },
  {
    blurb    : 'Yapf alias for `# fmt: off`.',
    effect   : 'Alias for `# fmt: off`, recognized so a project moving from yapf keeps its '
             + 'markers.',
    example  : '# yapf: disable\nkeep_this_block_exactly_as_written = (1,2,3)\n# yapf: enable',
    form     : '# yapf: disable',
    id       : 'yapf-disable',
    pairId   : 'yapf-enable',
    pairRole : 'opens',
    scope    : 'block'
  },
  {
    blurb    : 'Yapf alias for `# fmt: on`.',
    effect   : 'Alias for `# fmt: on`. Closes a yapf-style exempt region.',
    example  : '# yapf: disable\nkeep_this_block_exactly_as_written = (1,2,3)\n# yapf: enable',
    form     : '# yapf: enable',
    id       : 'yapf-enable',
    pairId   : 'yapf-disable',
    pairRole : 'closes',
    scope    : 'block'
  },
  {
    blurb   : 'Keep this dict or dunder list in the order written.',
    effect  : 'Every ordering rule leaves the entries in the order written, read from the opening '
            + 'or the closing bracket line. Covers that one dict literal, `__all__`, or `__slots__`.',
    example : 'config = {  # prose: keep\n    "stage_one"   : True,\n    "stage_two"   : '
            + 'False,\n}',
    form    : '# prose: keep',
    id      : 'prose-keep',
    scope   : 'dict'
  },
  {
    blurb   : 'Exempt the logical line it ends from every rewrite.',
    effect  : 'Every auto-fix rule skips the logical line the directive ends. Add '
            + '`[<rule>, ...]` to narrow it to the named rules. Lint findings still report.',
    example : 'data = {"a": 1, "b": 2, "c": 3}  # fmt: skip',
    form    : '# fmt: skip',
    id      : 'fmt-skip',
    scope   : 'line'
  },
  {
    blurb   : 'Alias for `# fmt: skip`.',
    effect  : 'Alias for `# fmt: skip`. Every auto-fix rule skips the logical line it ends.',
    example : 'data = {"a": 1, "b": 2, "c": 3}  # prose: skip',
    form    : '# prose: skip',
    id      : 'prose-skip',
    scope   : 'line'
  },
  {
    blurb   : 'Exempt the line from the listed rewrite rules only.',
    effect  : 'Only the listed auto-fix rules skip that logical line. Two bracketed directives '
            + 'on one line combine their rule slugs.',
    example : 'foo = 1  # prose: skip[align-equals, strip-trailing-commas]',
    form    : '# prose: skip[<rule>, ...]',
    id      : 'prose-skip-rules',
    scope   : 'line'
  },
  {
    blurb   : 'Silence every lint on the line.',
    effect  : 'Every lint rule skips the line. Add `[<rule>, ...]` to narrow it to the named '
            + 'rules.',
    example : 'helper = build_helper()  # prose: ignore',
    form    : '# prose: ignore',
    id      : 'prose-ignore',
    scope   : 'line'
  },
  {
    blurb   : 'Silence the listed lints on the line.',
    effect  : 'Only the listed lint rules skip the line. Two bracketed directives on one line '
            + 'combine their rule slugs.',
    example : 'TIMEOUT = 30  # prose: ignore[reassigned-constants, inlinable-bindings]',
    form    : '# prose: ignore[<rule>, ...]',
    id      : 'prose-ignore-rules',
    scope   : 'line'
  }
]
