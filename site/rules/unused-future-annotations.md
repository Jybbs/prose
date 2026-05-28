---
category : auto-fix
family   : formatting
caption  : "removes `from __future__ import annotations` lines that no longer carry their weight on the target Python version."
related  : [legacy-union-syntax]
layout   : doc
---

# unused-future-annotations

<RuleLayout rule="unused_future_annotations">

The `from __future__ import annotations` directive made forward-reference annotations possible on Python versions where the runtime evaluated annotations eagerly. PEP 749 lands deferred annotation evaluation in Python **3.14** by default whenever the file's annotations are typing-only, and the import becomes redundant. `unused-future-annotations` removes the import when removal is provably safe for the file.

Three branches actually fire the rewrite. The file may carry zero annotations (*the directive is unused outright*). The `target-version` may be 3.14 or higher (*the runtime defers annotation evaluation, so the directive carries no runtime weight*). Or every name appearing in every annotation may resolve to a module-scope binding before its first annotation use (*forward references aren't needed, so the runtime evaluates annotations eagerly without raising*). When none of those branches holds, the import stays in place.

::: tabs key:prose-target-version
== Python 3.10
The version-gated branch stays quiet. Removal fires only if the file has zero annotations or every annotation resolves to a module-scope binding before use.

== Python 3.11
Same as 3.10.

== Python 3.12
Same as 3.10.

== Python 3.13
Same as 3.10.

== Python 3.14
The version-gated branch fires. PEP 749 lands deferred annotation evaluation, so the directive is redundant for typing-only annotations and the import removes cleanly.
:::

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates the rewrite per project.

</template>

<template #canonical-lead>

A file whose annotations are typing-only loses the `__future__` import when the target version allows safe removal.

</template>

<template #related-after>

For the gate semantics, [**`target-version`**](/reference/configuration#top-level-keys) in the Configuration chapter covers how the field is read across version-gated rules.

</template>

</RuleLayout>
