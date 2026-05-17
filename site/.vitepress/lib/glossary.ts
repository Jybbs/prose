export interface GlossaryEntry {
  definition : string
  href      ?: string
}

export const glossary: Record<string, GlossaryEntry> = {
  'auto-fix': {
    definition: 'A rule category whose diagnostics rewrite source under `prose format` and surface as `Severity::AutoFix` under `prose check`.'
  },

  'BindingAnalysis': {
    definition: 'Per-`Source` table indexing every write and read of every name in every lexical scope. The primitive `single-use-variables` consumes.',
    href      : '/primitives/binding-analysis'
  },

  'code-line-length': {
    definition: 'Top-level config key for the line budget consumed by code-shaped rules. Defaults to **88**.',
    href      : '/guide/configuration#top-level-keys'
  },

  'docstring-line-length': {
    definition: 'Top-level config key for the description-prose budget inside docstrings. Defaults to **76**.',
    href      : '/guide/configuration#top-level-keys'
  },

  'lint': {
    definition: 'A rule category whose diagnostics surface as `Severity::Lint` without rewriting source. Always inspected, never modified.'
  },

  'max-shift': {
    definition: 'Per-alignment-rule config key capping per-line padding. Defaults to **8**. Groups whose widest member exceeds the cap fall back to `max-shift-policy`.',
    href      : '/guide/configuration#per-rule-knobs'
  },

  'max-shift-policy': {
    definition: 'How an alignment group overflowing `max-shift` resolves. `split` partitions the group, `drop` excludes the widest members, `skip` leaves the whole group unaligned.',
    href      : '/guide/configuration#per-rule-knobs'
  },

  'Pipeline': {
    definition: 'Orchestrates the rule loop against a `Source`, reparses between rules, returns the final source plus diagnostics.',
    href      : '/primitives/pipeline'
  },

  'RuleId': {
    definition: 'Canonical kebab-case slug identifying each registered rule across CLI flags, config tables, suppression directives, and diagnostic output.',
    href      : '/primitives/rule-id'
  },

  'singleton rule': {
    definition: 'Rule that drops alignment padding when a group resolves to a single member, so a one-key dict reads as plain code.',
    href      : '/rules/singleton-rule'
  },

  'Source': {
    definition: 'Parsed-text wrapper bundling original text, AST, token stream, line index, and suppression map. Every rule reads through this value.',
    href      : '/primitives/source'
  },

  'SuppressionMap': {
    definition: 'Per-`Source` index of `# fmt: off` / `# fmt: skip` / `# yapf` / `# prose: ignore[...]` directives, consulted at the edit-emission boundary.',
    href      : '/primitives/suppression-map'
  },

  'target-version': {
    definition: 'Top-level config key naming the Python runtime the project ships to. Consumed by version-gated rules. Unset means no version-dependent rewrites fire.',
    href      : '/guide/configuration#top-level-keys'
  }
}
