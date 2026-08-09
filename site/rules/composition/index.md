# Rule Composition

Where a per-rule page walks one rule's canonical case in isolation, each case here pairs a small Python source with the rule set it activates and shows the combined effect of those rules running together in [**Pipeline Order**](/reference/pipeline-order). They are the same `crate/tests/fixtures/composition/` set the binary's integration tests run against.

## The Canonical Case

One module-level constant puts a rule from each family in motion:

- Values use the legacy `Union[…]` form, so [[modernize-annotations]] rewrites them to the `|` operator and retires the `typing` import they read through.
- The literal overflows `code-line-length` on a single line, so [[collection-layout]] breaks it apart.
- Entries arrive in authorship order rather than alphabetical, so [[alphabetize]] sorts them.
- Keys line up against a vertical column, so [[align-colons]] computes the padding against the widths the rewrite leaves.
- The retired import leaves a gap where it stood, so [[blank-lines]] closes it.

The rules fire against the same block, reparsing between each so every rule downstream measures the rewritten source.

<Fixture rule="composition" case="overflow_dict_constants_modernize_unions" />

## All Cases

<CompositionCards />

## How Composition Resolves

Each case runs its listed rules in canonical order, and the shapes below are the common interactions.

### Layout Before Alignment

[[collection-layout]] running upstream of [[align-colons]] commits the per-line shape against which the alignment columns are computed.

### Reorder Before Align

[[alphabetize]] running upstream of [[align-equals]] settles the entry order, meaning the alignment math measures against the final column positions rather than the source ones.

### Docstring Discipline Before Wrap

[[docstring-expand]] and [[docstring-frame]] running upstream of [[docstring-wrap]] settle the quote placement before the body rewrap measures budgets.

### Module Reorder Around a Block Marker

[[alphabetize]]'s module-level branch reorders the assigns above and below a `# fmt: off` block while the bracketed lines stay verbatim, so both it and [[align-equals]] fire freely outside the bracket.

<Fixture rule="composition" case="constants_sort_around_fmt_off" />

Click any rule chip above for its canonical case, the [**Pipeline Order**](/reference/pipeline-order) reference for the order the pipeline runs in, the [[pipeline]] primitive for the runner, and the [**Rules**](/rules/) catalog for the rest.
