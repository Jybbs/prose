---
category: lint
related : [loose-constants, no-step-narration]
---

# single-use-variables

A binding that's assigned once and read once usually exists because the author wanted a name for the expression, and the name reads better than the expression at the call site. Sometimes that's a real win, and sometimes the binding is just standing in for inlining the right-hand side. *Single-use-variables* surfaces bindings assigned and read exactly once, leaving the inline-or-keep decision to a future refactor pass that picks up the lint output.

The rule consumes the per-`Source` [[binding-analysis]] table to count writes and reads per binding. Bindings matching the `allow-pattern` regex (*defaulting to `^_`, which exempts intentionally-unused names*) stay quiet. Augmented assignments count as both a write and a read, so a binding they target isn't single-use. Loop variables, comprehension targets, and function parameters are introduced implicitly and stay outside the rule's reach. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

## Configuration

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow-pattern` | regex | `"^_"` | Binding names exempted from the lint |

The default `^_` exempts names starting with an underscore, matching the Python convention for intentionally-unused bindings. Projects with stricter naming can tighten the regex.

## The Canonical Case

A binding assigned and read exactly once surfaces the lint, recommending inlining the right-hand side.

<Fixture rule="single_use_variables" case="basic_flag" />

## More Examples

<Fixture rule="single_use_variables" case="closure_capture_flagged" title="A Closure-Captured Single-Use Binding Is Flagged" />

<Fixture rule="single_use_variables" case="async_function_flagged" title="Async Function Bodies Are Recognized the Same Way" />

<Fixture rule="single_use_variables" case="augmented_skipped" title="Augmented Assignments Count as Both Write and Read" />

<Fixture rule="single_use_variables" case="comprehension_target_skipped" title="Comprehension Targets Stay Outside the Rule's Reach" />

<Fixture rule="single_use_variables" case="global_function_skipped" title="Global-Scope Function Bindings Aren't Single-Use" />

<Fixture rule="single_use_variables" case="configured_allow" title="A Custom `allow-pattern` Exempts Matching Names" />

<Fixture rule="single_use_variables" case="fmt_off_suppresses" title="A `# fmt: off` Block Suppresses the Lint" />

## Related

The binding-shaped lint composes with two other surfaces that consume the same analysis.

- [[binding-analysis]] is the per-`Source` table this rule reads, where every write and read of every name in every scope is indexed once and queried by consuming rules.
- [[loose-constants]] lints the module-level equivalent (`SCREAMING_CASE = literal` assignments that would read better as a structured shape).

For per-line opt-outs, the [**Suppression**](/guide/suppression#lint-directives) chapter covers the `# prose: ignore[single-use-variables]` directive.
