---
caption : "Reports a `skip` or `ignore` directive that holds nothing out."
related : [line-overflow]
layout  : doc
---

# inert-directives

<RuleLayout rule="inert_directives">

`inert-directives` reports a `skip` or `ignore` directive that holds nothing out of the run checking it, so a reader of the source can tell which directives still do their job. A directive goes on holding nothing once the code it covered changes or a rule it names is renamed, and one written in the family that does not reach its rule, such as a `skip` meant to silence a lint, never held anything. Each report names the directive and the reason it holds nothing, covering:

- A `skip` whose removal leaves every rule it names making the same edits, reading `# fmt: skip` as a bare `skip`. A skip on one row of an alignment run still holds something where its own row would stay as written, as long as removing it moves the column an aligning rule pads the other rows to
- An `ignore` whose line carries no finding of a rule it names
- A bracketed id naming no registered rule, such as a misspelled or retired slug, once per id

A listed id naming a rule the run leaves out, through `--select`, `--ignore`, or the configuration, spares its directive, since a run that selects the rule may need it. A bare `skip` or `ignore` names no rule, so it is judged against the rules the run carries, and a run narrowed by `--select` to the lints reports every bare `skip`.

A bare `# prose: ignore` silences every lint on its line, so the rule reports one that does silence findings too, naming the slugs it silences and the bracketed form that holds out the same findings. A report on an `ignore` stands outside that directive's reach, so a bare `ignore` never silences the report on itself, whereas a `# prose: ignore[inert-directives]` on a line carrying a `skip` silences the report on that skip and counts as holding it out.

A directive inside a `# fmt: off` region is not reported, since the region silences every lint on its lines.

<template #related-after>

The [**Suppression Directives**](/reference/suppression-directives) reference gives the syntax of every `skip` and `ignore` form, and the [**Suppression**](/guide/suppression#tagging-a-line) chapter covers which family reaches a rewrite and which a lint.

</template>

</RuleLayout>
