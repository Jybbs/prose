---
caption : "Reports an own-line comment that narrates a numbered step."
related : [inlinable-bindings, reassigned-constants]
layout  : doc
---

# step-narration

<RuleLayout rule="step_narration">

`step-narration` reports an own-line numbered-step comment (*`# 1. ...`, `# Step 2: ...`*), because such a comment inside a function body usually stands in for the helper name that step never got, and it leaves the extract-to-helper decision to whoever reads the finding.

Two forms are recognized, the bare numeric-dot form `# N. text` and the `Step`-prefixed forms `# Step N: text` and `# Step N. text` (*with the keyword written `Step` or `step`*). An inline comment at the end of a code line stays quiet, because it annotates the line rather than narrating a procedure. A pragma comment (*`# type: ignore`, `# noqa`*) stays quiet too, since it carries a different meaning, and a decimal version such as `# 1.2 ...` matches neither form. The lint fires at every scope (*module level, function body, class body, nested block*) and never rewrites.

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#tagging-a-line) chapter covers the `# prose: ignore[step-narration]` directive.

</template>

</RuleLayout>
