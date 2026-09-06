---
caption : "Removes a bare `-> None` return annotation, since an omitted one already reads as returning nothing."
related : [prune-inert-imports, signature-annotations, reflow-signatures]
layout  : doc
---

# strip-none-return

<RuleLayout rule="strip_none_return">

`strip-none-return` removes a written `-> None` from a function that returns nothing, so `def configure() -> None:` reads `def configure():`. An omitted return annotation already reads as a function that returns nothing, which leaves the explicit form as visual weight rather than information.

The rewrite is purely mechanical, running only where the return annotation is a bare `None`, with or without its own parentheses, and leaving a `None` nested inside a larger annotation *(`int | None`, `Callable[..., None]`)* and every parameter annotation as written. A declaration-only stub keeps its `-> None` too, because a body that is only `...`, with or without a docstring ahead of it *(an `@overload` arm, a `Protocol` method, an abstract method)*, is a placeholder whose `-> None` declares a type-checker contract rather than a redundant annotation. The companion [[signature-annotations]] rule enforces the other side of the convention, reporting where a parameter or a value-returning function lacks the annotation it owes.

<template #configuration>

<RuleConfigTable />

</template>

<template #related-after>

For per-statement opt-outs, the [**Suppression**](/usage/suppression) chapter covers the `# prose: skip[strip-none-return]` directive, which covers every line a wrapped statement spans.

</template>

</RuleLayout>
