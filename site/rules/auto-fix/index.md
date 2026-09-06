# Auto-Fix Rules

An auto-fix rule rewrites source under `prose format` and reports each rewrite as a `Severity::Format` diagnostic under `prose check`. Each rule settles one layout question *Prose* can answer mechanically *(an alignment column, an alphabetical order, a blank-line count, a collection layout, a trailing comma)* and emits an [[edit]] list the [[pipeline]] applies between rules. An auto-fix rule never reports a change the binary will not itself write.

<RuleCardList category="auto-fix" />

The [**Pipeline Order**](/reference/pipeline-order) reference lists the fixed order these rules run in, the [**Configuration**](/reference/configuration) reference lists the per-rule facets, and the [**Lint**](/rules/lint/) landing covers the rules that report a diagnostic without rewriting.
