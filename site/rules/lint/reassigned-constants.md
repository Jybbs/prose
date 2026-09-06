---
caption : "Reports a module-level `SCREAMING_CASE` name the module assigns more than once."
related : [inlinable-bindings, miscased-constants, step-narration]
layout  : doc
---

# reassigned-constants

<RuleLayout rule="reassigned_constants">

`reassigned-constants` reports a module-level `SCREAMING_CASE` binding the module reassigns, because the casing promises a constant and a second write contradicts it. A binding counts as reassigned when the binding table records more than one write against the name or an augmented assignment, and a write-once constant stays silent whatever its value. The fix is to rename the variable to lowercase or to stop reassigning it, and the lint leaves that work to a later migration pass that reads its output.

The rule reads module-level `SCREAMING_CASE` assignments and annotated assignments and reports only the reassigned ones, whereas several kinds of binding stay quiet:

- A name on the configurable `allow` list.
- A dunder name (*`__version__`, `__all__`*), which falls outside `SCREAMING_CASE` because it leads with an underscore.
- A typing construct from the standard library (*`TypeVar`, `ParamSpec`, `NewType`, `TypeAliasType`*) and any binding declared inside an `if TYPE_CHECKING:` block, because both carry semantics of their own distinct from runtime configuration.
- In-place mutation through a method call or a subscript store, which the binding table records as a read, so it stays out of scope.

The lint never rewrites, so the diagnostic is reported and the source stays as written.

<template #configuration>

<RuleConfigTable />

The `allow` list takes bare names, and a listed name never produces a finding even when it would otherwise match.

</template>

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[reassigned-constants]` directive.

</template>

</RuleLayout>
