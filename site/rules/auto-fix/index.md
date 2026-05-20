# Auto-Fix Rules

The **fourteen** auto-fix rules rewrite source as part of `prose format` and surface as `Severity::Format` diagnostics under `prose check`. Each rule resolves a layout question *Prose* can answer mechanically *(alignment columns, alphabetization order, blank-line counts, collection layout, trailing-comma presence)* and emits an [[edit]] list the [[pipeline]] applies between rules. Auto-fix rules never report a violation the binary won't itself resolve.

<RuleCardGrid category="auto-fix" />

For the deterministic order these rules fire in, see the [**Pipeline Order**](/reference/pipeline-order) reference. For the per-rule knobs, see the [**Configuration**](/reference/configuration) reference. For the lint rules that surface diagnostics without rewriting, see the [**Lint**](/rules/lint/) landing.
