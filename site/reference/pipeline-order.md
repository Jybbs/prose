---
description: "Covers the fixed order rules run in, why each rule sits where it does, and what the corpus sweep holds each rule to."
---

# Pipeline Order

*Prose* runs its enabled rules in a fixed order, re-parsing the source between batches of independent rules so every later rule reads a tree that reflects the earlier rules' edits. That re-parse is what lets the rules combine, since no rule ever reads a file with another rule's edits half applied, and no two rules' edits conflict within one pass. The order is defined in one place, the `register_rules!` block in `crate/src/rules/registry.rs`, and reading it explains the pipeline. A rule that reads text an earlier rule rewrites runs after that rule. [[align-colons]] runs before [[wrap-docstrings]], for example, because the docstring wrap budget depends on the column the alignment rule sets after each colon.

## Canonical Order

<PipelineOrder />

## Why Ordering Matters

Each rule's edits change the source the next rule reads. Three kinds of dependency set the order.

### Layout Before Alignment

[[reflow-collections]] runs near the top because the rules after it *(alignment, alphabetization)* read the one-entry-per-line layout it writes. Aligning first would pad lines that the next pass joins back together.

### Reorder Before Align

[[alphabetize-siblings]] runs before the alignment rules, so the columns the aligners compute reflect the final order of the entries rather than the source order.

### Strip Before Pad

[[strip-trailing-commas]] runs before alignment, so the trailing comma is already decided when alignment measures each row's width. Padding a line that is about to lose its trailing comma would put the column in the wrong place.

Each of those orderings relies on the re-parse between batches, which lets a rule that reads text an earlier rule rewrites see that text in the tree it reads. A rule the registry declares independent of every rule in the current batch instead reads the buffer the batch opened on and applies its edits in the same pass, so the cost is one parse per batch rather than one per editing rule. Whenever no rule in the batch changed a binding, the binding analysis carries across that parse rather than being rebuilt, whereas the layout forecasts are rebuilt after any batch that edits.

## Every Subset Settles

The order guarantees more than the default set settling a file in one pass. Any subset a project enables settles too, whether through `--select`, `--ignore`, or a rule turned off under `[tool.prose.rules]`, because a subset that needed a second pass would be a defect for whoever configured it rather than a curiosity. A corpus sweep in CI checks the guarantee, so a rule that depends on a later rule to finish its work is caught where the fault is rather than hidden by the default pipeline.

The guarantee needs no sweep over every subset, because a rule that settles alone and never unsettles an earlier rule leaves every larger subset settled, so checking each rule alone and each ordered pair of rules covers all of them.

Each ordering the guarantee depends on is recorded in the registry's dependency column rather than left to a position that happens to work, and `prose rules --output-format json` prints that column as each rule's `after` list.

## Rules That Keep the Tree

Many rules change only whitespace, parentheses, commas, comments, and how a literal is spelled, as [[align-equals]] does when it pads a column and [[normalize-literals]] does when it settles a string on `"`. A rewrite confined to those leaves the parsed tree as it was, in a comparison that ignores positions, parentheses, comments, and implicit string concatenation. The corpus sweep holds each such rule to that comparison on every rewrite it makes, and runs every such rule together in one pipeline under the same comparison. The comparison reads every function body in the corpus, including code no import of the module ever runs, so a rule that changed what a line does fails the sweep before it reaches a release.

A rule whose rewrite changes the tree sits outside that check, even where the rewritten code runs exactly as the original did:

- [[reflow-imports]] splits `import a, b` into two statements
- [[strip-none-return]] drops a `-> None` annotation
- [[reflow-calls]] writes a positional argument in keyword form, named after its parameter
- [[wrap-docstrings]] and [[align-colons]] change the text inside a docstring, which the tree holds as a string value

Those rules answer to a second sweep, which executes every module *Prose* rewrote and compares what each one binds before and after formatting.

## Independent Rules Share a Parse

A run applies the edits of consecutive rules whose edits are independent to one buffer and parses once. The batch closes before a rule the registry places after one the batch holds, and before a rule whose edits overlap one already in the batch. Independence is declared rather than assumed, in a shared-splice column each rule carries beside its dependency column in the registry, and a pair joins that column on two kinds of evidence:

1. The subset probe finding the two rules editing a standard-library file together, with the batched splice matching the rule-by-rule result on every such file at every line length.
2. A reading of the later rule's `apply` that finds nothing it measures among what the earlier rule rewrites, meaning the text a column is computed from, the adjacency of the rows a run spans, a statement's position, a name binding, or a docstring's rows.

A row's fit against the budget is left to the probe alone, so a rule that measures only that shares a splice with one rewriting the row's value side. The probe re-checks every declared pair on each `cargo test` over the fixture tree and on every corpus sweep. It runs the pair both ways, splicing both rules into one buffer and then running them one after the other, and fails where the two runs produce different text. A batch whose combined edit the re-parse rejects replays its rules one at a time, so the failure still names the rule whose own edits caused it.

## Lint Rules

Lint rules *(the entries above with the 🧶 badge)* never rewrite, so they do not change the source the next rule reads. They could run in any order, and they sit at fixed positions so the registered set stays stable for the [`Pipeline::known_ids`](/primitives/pipeline) consumer and for the `--select` and `--ignore` flags.

## Internal Surface

The data on this page comes from running `prose rules --output-format json` at build time, so the order shown is always the order the binary runs. The [[pipeline]] primitive page covers the `Pipeline::with_defaults`, `Pipeline::with_filters`, and `Pipeline::for_rule` constructors that pick subsets out of this list.

Click a rule's chip above for its canonical case and the behavior around it. The [**Exit Codes**](/reference/exit-codes) reference covers the codes a CI gate reads.
