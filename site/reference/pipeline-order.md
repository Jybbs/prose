---
description: "Covers the fixed order rules run in, why each rule sits where it does, and what the corpus sweep holds each rule to."
---

# Pipeline Order

*Prose* runs its enabled rules in a fixed order, re-parsing the source between batches of independent rules so every rule reads a tree that reflects the edits of every batch before its own, as the [[pipeline]] page sets out. That re-parse is what lets the rules combine, since no rule ever reads a file with another rule's edits half applied, and no two rules' edits conflict within one pass. The order is defined in one place, the `register_rules!` block in `crate/src/rules/registry.rs`, and reading it explains the pipeline. A rule that reads text an earlier rule rewrites runs after that rule. [[align-colons]] runs before [[wrap-docstrings]], for example, because the docstring wrap budget depends on the column the alignment rule sets after each colon.

## Canonical Order

<PipelineOrder />

## Order Dependencies

Each rule's edits change the source the next rule reads. The dependencies below set the order.

### Layout Before Alignment

[[reflow-collections]] runs near the top because the rules after it *(alignment, alphabetization)* read the rows it writes. Aligning first would pad lines that the next pass joins back together.

### Reorder Before Align

[[alphabetize-siblings]] runs before the alignment rules whose rows it reorders, so the columns the aligners compute reflect the final order of the entries rather than the source order.

### Strip Before Pad

[[strip-trailing-commas]] runs before alignment, so the trailing comma is already decided when alignment measures each row's width. Padding a line that is about to lose its trailing comma would put the column in the wrong place.

## Every Subset Settles

The order guarantees more than the default set settling a file in one pass. Any subset a project enables settles too, whether through `--select`, `--ignore`, or a rule turned off under `[tool.prose.rules]`, because a subset that needed a second pass would be a defect for whoever configured it rather than a curiosity. A corpus sweep in CI checks the guarantee, so a rule that depends on a later rule to finish its work is caught where the fault is rather than hidden by the default pipeline.

The guarantee needs no sweep over every subset, because a rule that settles alone and never unsettles an earlier rule leaves every larger subset settled, so checking each rule alone and each ordered pair of rules covers all of them.

Each ordering the guarantee depends on is recorded in the registry's dependency column rather than left to a position that happens to work, and `prose rules --output-format json` prints that column as each rule's `after` list.

## Rules That Keep the Tree

Many rules change only whitespace, parentheses, commas, comments, and how a literal is spelled, as [[align-equals]] does when it pads a column and [[normalize-literals]] does when it settles a string on `"`. A rewrite confined to those leaves the parsed tree as it was, in a comparison that ignores positions, parentheses, comments, and implicit string concatenation. The corpus sweep holds each such rule to that comparison on every rewrite it makes, and runs every such rule together in one pipeline under the same comparison. The comparison reads every function body in the corpus, including code no import of the module ever runs, so a rule that changed what a line does fails the sweep before it reaches a release. The rules below declare that they keep the tree, listed in pipeline order:

<TreeRules tree="kept" />

A rule whose rewrite changes the tree sits outside that check, even where the rewritten code runs exactly as the original did, as these examples do:

- [[reflow-imports]] splits `import a, b` into two statements
- [[strip-none-return]] drops a `-> None` annotation
- [[reflow-calls]] writes a positional argument in keyword form, named after its parameter
- [[wrap-docstrings]] and [[align-colons]] change the text inside a docstring, which the tree holds as a string value

The rules below can change the tree, so they answer to a second sweep instead, which executes every module *Prose* rewrote and compares what each one binds before and after formatting:

<TreeRules tree="changed" />

## Lint Rules

Lint rules *(the entries above with the 🧶 badge)* never rewrite, so they do not change the source the next rule reads. They could run in any order, and they sit at fixed positions so the registered set stays stable for the [`Pipeline::known_ids`](/architecture/pipeline) consumer and for the `--select` and `--ignore` flags.

## Internal Surface

The data on this page comes from running `prose rules --output-format json` at build time, so the order shown is always the order the binary runs. The [[pipeline]] primitive page covers the `Pipeline::with_defaults`, `Pipeline::with_filters`, and `Pipeline::for_rule` constructors that pick subsets out of this list.

Click a rule's chip above for its canonical case and the behavior around it.
