---
category : lint
domain   : lint
caption  : "*Prose* surfaces comments that narrate the next line."
related  : [loose-constants, single-use-variables]
---

# no-step-narration

A numbered-step comment inside a function body (*`# 1. ...`, `# Step 2: ...`*) is usually a signal that the function is doing too many things. Each numbered step wants to be its own helper with a name that captures what the step does, and the comment is standing in for that name. `no-step-narration` surfaces own-line numbered-step comments as a lint, leaving the extract-to-helper decision to a future refactor pass.

Two shapes are recognized: the bare numeric-dot form `# N. text` and the `Step`-prefixed forms `# Step N: text` and `# Step N. text` (case-insensitive on the keyword). Inline comments at the end of a code line stay quiet, since they annotate the line rather than narrate a procedure. Pragma-style comments (*`# type: ignore`, `# noqa`*) stay quiet too, since they carry a different meaning. The lint fires at every scope (*module-level, function body, class body, nested block*) and never rewrites.

## Configuration

<RuleConfigTable preset="toggle" />

## The Canonical Case

A module-level own-line numbered-step comment surfaces the lint.

<Fixture rule="no_step_narration" case="module_level_step" />

## More Examples

<Fixture rule="no_step_narration" case="class_body_step" title="Class-Body Numbered Steps Surface the Lint Too" />

<Fixture rule="no_step_narration" case="numeric_dot_step" title="Numeric-Dot Shape (`# 1.`) Is Recognized" />

<Fixture rule="no_step_narration" case="multiple_steps" title="Multiple Steps in One Function Surface One Lint Each" />

<Fixture rule="no_step_narration" case="inline_comment_skipped" title="Inline End-of-Line Comments Stay Quiet" />

<Fixture rule="no_step_narration" case="pragma_skipped" title="Pragma-Style Comments Stay Quiet" />

<Fixture rule="no_step_narration" case="fmt_off_block" title="A `# fmt: off` Block Suppresses the Lint" />

<Fixture rule="no_step_narration" case="idempotent" title="Source Without Numbered Comments Surfaces Nothing" />

## Related

<RelatedRulesInline />

For per-line opt-outs, the [**Suppression**](/guide/suppression#lint-directives) chapter covers the `# prose: ignore[no-step-narration]` directive.
