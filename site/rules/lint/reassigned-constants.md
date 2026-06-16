---
caption : "Surfaces a module-level constant reassigned despite its `UPPER_SNAKE_CASE` casing."
related : [step-narration, single-use-variables]
layout  : doc
---

# reassigned-constants

<RuleLayout rule="reassigned_constants">

A `SCREAMING_CASE` name promises a constant, so a module-level one that is reassigned contradicts its own casing. `reassigned-constants` surfaces a module-level `SCREAMING_CASE` binding only when it is reassigned (*an `assignment_count` above one or an augmented assignment recorded against the name*), leaving a write-once constant silent whatever its value. The fix renames the variable to lowercase or stops reassigning it, work the lint leaves to a future migration pass that picks up its output.

The rule weighs module-level `SCREAMING_CASE` assignments and annotated assignments, firing only on the reassigned ones. Names on the configurable `allow` list stay quiet. Dunder-style names (*`__version__`, `__all__`*) fall outside `SCREAMING_CASE` because they lead with an underscore. Typing constructs from the standard library (*`TypeVar`, `ParamSpec`, `NewType`, `TypeAliasType`*) and any binding declared inside an `if TYPE_CHECKING:` block also stay quiet, since both carry their own semantics distinct from runtime configuration. In-place mutation through a method call or a subscript store stays out of scope, since the binding table records those as reads. The lint is non-rewriting, so the diagnostic surfaces without touching the source.

<template #configuration>

| Key | Type | Default | Meaning |
|---|---|---|---|
| `enabled` | bool | `true` | Toggle the rule on or off |
| `allow` | list of names | `[]` | Module-level names exempted from the lint |

The `allow` list holds bare names, so an entry never produces a lint even when its shape would otherwise match.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[reassigned-constants]` directive.

</template>

</RuleLayout>
