export interface GlossaryEntry {
  aliases   ?: readonly string[]
  definition : string
  href      ?: string
}

export const glossary: Record<string, GlossaryEntry> = {
  '# fmt: off': {
    aliases   : ['# fmt: on'],
    definition: 'Block markers that preserve the exact source layout of code between them by disabling every rewriting rule. Inline comments on the same line are recognized as the marker.',
    href      : '/guide/suppression#block-markers'
  },

  '# fmt: skip': {
    definition: 'Line-level marker that exempts the statement it sits on from every rewriting rule, without needing surrounding block markers.',
    href      : '/guide/suppression#line-markers'
  },

  '# prose: ignore': {
    aliases   : ['# prose: ignore[...]'],
    definition: 'Per-line directive that suppresses specific lint diagnostics. The bracketed form names the rule slugs to silence; the bare form silences every lint on that line.',
    href      : '/guide/suppression#lint-directives'
  },

  '--ignore': {
    definition: 'CLI flag that disables the named rules for a single invocation. Repeatable. Pairs with `--select` to scope a run.',
    href      : '/guide/quick-start#subset-the-active-rules'
  },

  '--select': {
    definition: 'CLI flag that restricts a run to the named rules. Repeatable. Pairs with `--ignore` to subtract from the active set.',
    href      : '/guide/quick-start#subset-the-active-rules'
  },

  'AST': {
    aliases   : ['abstract syntax tree'],
    definition: 'The parsed-program tree produced by `ruff_python_parser`. Bundled inside `Source` and reparsed between rules so each rule reads against the post-rewrite tree.',
    href      : '/primitives/source'
  },

  'BindingAnalysis': {
    aliases   : ['binding analysis', 'binding map', 'name bindings', 'binding', 'bindings'],
    definition: 'Per-`Source` table indexing every write and read of every name in every lexical scope. Consumed by `single-use-variables`.',
    href      : '/primitives/binding-analysis'
  },

  'Diagnostic': {
    aliases   : ['diagnostic', 'diagnostics', 'lint diagnostic'],
    definition: 'Structured report a rule emits when it detects a pattern. Carries severity (`AutoFix` rewrites source under `prose format`; `Lint` only surfaces).',
    href      : '/integrations/github-actions'
  },

  'Pipeline': {
    aliases   : ['pipeline'],
    definition: 'Orchestrates the rule loop against a `Source`, reparses between rules, and returns the final source plus diagnostics.',
    href      : '/primitives/pipeline'
  },

  'Ruff': {
    aliases   : ['ruff'],
    definition: 'Astral\'s Python linter and formatter. `prose` is designed to compose downstream of `ruff format`, leaving token-level normalization to `ruff` and layout-level legibility to `prose`.',
    href      : '/guide/two-stage-pipeline'
  },

  'RuleId': {
    aliases   : ['rule id', 'rule-id', 'rule IDs'],
    definition: 'Canonical kebab-case slug identifying each registered rule across CLI flags, config tables, suppression directives, and diagnostic output.',
    href      : '/primitives/rule-id'
  },

  'Source': {
    definition: 'Parsed-text wrapper bundling original text, AST, token stream, line index, and suppression map. Every rule reads through this value.',
    href      : '/primitives/source'
  },

  'SuppressionMap': {
    aliases   : ['suppression map', 'suppression directive', 'suppression directives', 'suppression'],
    definition: 'Per-`Source` index of `# fmt: off` / `# fmt: skip` / `# yapf` / `# prose: ignore[...]` directives, consulted at the edit-emission boundary.',
    href      : '/primitives/suppression-map'
  },

  'atomic': {
    aliases   : ['atomic literal', 'atomic literals'],
    definition: 'A simple, indivisible code element (integer, float, string, single name) that `collection-layout` can safely keep on one line without readability loss.',
    href      : '/rules/collection-layout'
  },

  'auto-fix': {
    aliases   : ['auto-fixes', 'auto-fixing', 'Auto-Fix'],
    definition: 'Rule category whose diagnostics rewrite source under `prose format` and surface as `Severity::AutoFix` under `prose check`.'
  },

  'blank line': {
    aliases   : ['blank-line', 'blank lines', 'blank-lines'],
    definition: 'Empty line separating logical units. `prose` enforces blank-line counts between module-level definitions, class members, and import groups per the `blank-lines` rule.',
    href      : '/rules/blank-lines'
  },

  'code-line-length': {
    definition: 'Top-level config key for the line budget consumed by code-shaped rules. Defaults to **88**.',
    href      : '/reference/configuration#top-level-keys'
  },

  'docstring': {
    aliases   : ['docstrings', 'triple-quoted docstring'],
    definition: 'A triple-quoted string literal placed as the first statement in a module, class, or function. `prose` rewraps multi-line bodies under `docstring-wrap` and gates single-line shapes under `no-single-line-docstrings`.',
    href      : '/rules/docstring-wrap'
  },

  'docstring-line-length': {
    definition: 'Top-level config key for the description-prose budget inside docstrings. Defaults to **76**.',
    href      : '/reference/configuration#top-level-keys'
  },

  'lint': {
    aliases   : ['Lint', 'lint violation', 'lint-only', 'linting'],
    definition: 'Rule category whose diagnostics surface as `Severity::Lint` without rewriting source. Always inspected, never modified.'
  },

  'max-shift': {
    definition: 'Per-alignment-rule config key capping per-line padding. Defaults to **8**. Groups whose widest member exceeds the cap fall back to `max-shift-policy`.',
    href      : '/reference/configuration#per-rule-knobs'
  },

  'max-shift-policy': {
    definition: 'How an alignment group overflowing `max-shift` resolves. `split` partitions the group, `drop` excludes the widest members, `skip` leaves the whole group unaligned.',
    href      : '/reference/configuration#per-rule-knobs'
  },

  'ruff_python_parser': {
    definition: 'The Astral parser crate `prose` consumes to produce the AST inside each `Source`. Reparsing between rules guarantees every rule reads against the post-rewrite tree.'
  },

  'singleton rule': {
    aliases   : ['singleton rules'],
    definition: 'Rule that drops alignment padding when a group resolves to a single member, so a one-key dict reads as plain code.',
    href      : '/rules/singleton-rule'
  },

  'target-version': {
    aliases   : ['target version'],
    definition: 'Top-level config key naming the Python runtime the project ships to. Consumed by version-gated rules. Unset means no version-dependent rewrites fire.',
    href      : '/reference/configuration#top-level-keys'
  }
}

export function buildPhraseToSlug(source: Record<string, GlossaryEntry>): Map<string, string> {
  const out = new Map<string, string>()
  for (const [slug, entry] of Object.entries(source)) {
    register(out, slug, slug)
    for (const alias of entry.aliases ?? []) {
      register(out, alias, slug)
    }
  }
  return out
}

function register(map: Map<string, string>, phrase: string, slug: string): void {
  const existing = map.get(phrase)
  if (existing !== undefined && existing !== slug) {
    throw new Error(`Glossary phrase "${phrase}" registered against both "${existing}" and "${slug}"`)
  }
  map.set(phrase, slug)
}
