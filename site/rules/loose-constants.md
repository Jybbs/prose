---
category: lint
related : [single-use-variables, no-step-narration]
---

# loose-constants

Module-level `SCREAMING_CASE` constants tend to accumulate at the top of a file, and what starts as one or two related names grows into a cluster that would read better as an `Enum`, a model field, or a function-local. *Loose-constants* surfaces every module-level `SCREAMING_CASE` assignment as a lint candidate, leaving the refactor to a future migration pass that picks up the lint output.

The rule fires on bare module-level `SCREAMING_CASE = literal` assignments. Names on the configurable `allow` list stay quiet. Dunder-style names (*`__version__`, `__all__`*) are recognized as runtime sentinels rather than configuration. Typing constructs from the standard library (*`TypeVar`, `ParamSpec`, `NewType`, `TypeAliasType`*) and any binding declared inside an `if TYPE_CHECKING:` block also stay quiet, since both carry their own semantics distinct from runtime configuration. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

## Configuration

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow` | list of names | `[]` | Module-level names exempted from the lint |

The `allow` list holds bare names. An entry never produces a lint, even when its shape would otherwise match.

## The Canonical Case

A bare `SCREAMING_CASE = literal` at module level surfaces the lint, recommending a refactor to a more structured shape.

<Fixture rule="loose_constants" case="basic_flag" />

## More Examples

<Fixture rule="loose_constants" case="configured_allow" title="Names on the Configured Allow List Stay Quiet" />

<Fixture rule="loose_constants" case="dunder_skipped" title="Dunder Names Like `__all__` Are Recognized as Runtime Sentinels" />

<Fixture rule="loose_constants" case="ann_assign" title="Annotated Constants Are Flagged the Same Way" />

<Fixture rule="loose_constants" case="chained_and_tuple_targets" title="Chained and Tuple Targets Are Recognized" />

<Fixture rule="loose_constants" case="dotted_type_checking" title="Dotted `typing.TYPE_CHECKING`-Style Names Stay Quiet" />

<Fixture rule="loose_constants" case="fmt_off_block" title="A `# fmt: off` Block Suppresses the Lint" />

<Fixture rule="loose_constants" case="idempotent" title="Already-Conforming Source Surfaces Nothing" />

## Related

The lint-only surface composes with the other lints across the same source.

- [[single-use-variables]] lints function-local bindings assigned and read exactly once.
- [[no-step-narration]] lints numbered-step comments that signal extractable functions.

For per-line opt-outs, the [**Suppression**](/guide/suppression#lint-directives) chapter covers the `# prose: ignore[loose-constants]` directive.
