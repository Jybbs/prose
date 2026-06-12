---
caption : "surfaces comments that narrate the next line."
related : [reassigned-constants, single-use-variables]
layout  : doc
---

# step-narration

<RuleLayout rule="step_narration">

A numbered-step comment inside a function body (*`# 1. ...`, `# Step 2: ...`*) is usually a signal that the function is doing too many things. Each numbered step wants to be its own helper with a name that captures what the step does, and the comment is standing in for that name. `step-narration` surfaces own-line numbered-step comments as a lint, leaving the extract-to-helper decision to a future refactor pass.

Two shapes are recognized: the bare numeric-dot form `# N. text` and the `Step`-prefixed forms `# Step N: text` and `# Step N. text` (*case-insensitive on the keyword*). Inline comments at the end of a code line stay quiet, since they annotate the line rather than narrate a procedure. Pragma-style comments (*`# type: ignore`, `# noqa`*) stay quiet too, since they carry a different meaning. The lint fires at every scope (*module-level, function body, class body, nested block*) and never rewrites.

<template #related-after>

For per-line opt-outs, the [**Suppression**](/usage/suppression#lint-directives) chapter covers the `# prose: ignore[step-narration]` directive.

</template>

</RuleLayout>
