# Pipeline Order

*Prose* runs each enabled rule in a deterministic order, reparsing the source between rules so every downstream rule reads a settled AST. The reparse is the discipline that makes twenty rules composable, wherein no rule observes the half-applied state of another, leaving every pass free of cross-rule edit conflict by construction. The order itself is canonical, source-of-truth in `src/rule.rs` *(the `register_rules!` macro block)*, and pedagogically valuable. A rule that depends on a settled token surface sits downstream of every rule that touches that surface, in that *(for example)* [[align-colons]] runs before [[docstring-wrap]] because the docstring wrap budget depends on the post-colon column the alignment rule sets.

## The Order

<PipelineOrder />

## Why Ordering Matters

Each rule's edits shape the source the next rule reads. Three kinds of dependency drive the ordering.

### Layout Before Alignment

[[collection-layout]] runs near the top because rules below it *(alignment, alphabetization)* operate on the per-line shape collection-layout commits to. Aligning before laying out would pad lines that re-collapse in the next pass.

### Reorder Before Align

[[alphabetize]] runs before the alignment rules wherein the columns the aligners compute against reflect the final order of the entries rather than the source order.

### Strip Before Pad

[[strip-trailing-commas]] runs before alignment so the trailing-comma decision is settled when alignment math measures member widths. Padding a line that's about to lose its trailing comma would land at the wrong column.

The pipeline reparses between rules, so a rule that depends on a token surface earlier in the order sees that surface in the AST it walks. The cost is one parse per rule transition, paid against the marginal benefit of a clean borrow-stable input to each rule.

## Lint Rules

Lint-only rules *(the entries above with the 🧶 badge)* never rewrite, so they don't shape the source the next rule reads. They could in principle run in any order, but they sit at their canonical positions to make the registered set stable for the [`Pipeline::known_ids`](/primitives/pipeline) consumer and for the CLI's `--select` / `--ignore` ergonomics.

## Internal Surface

The data driving this page comes from parsing the `register_rules!` macro at `src/rule.rs` at build time, so the order on the page is always the order the binary actually runs. The [[pipeline]] primitive page covers the `Pipeline::with_defaults`, `Pipeline::with_filters`, and `Pipeline::for_rule` constructors that pick subsets out of this canonical list.

For the per-rule canonical case and the surrounding behavior of each entry, click the rule's chip above. For the deterministic gate that consumers compile against, see the [**Exit Codes**](/reference/exit-codes) reference.
