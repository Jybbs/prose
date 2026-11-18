---
description: "Covers `# fmt: off / on`, `# fmt: skip`, the `# yapf` aliases, `# prose: ignore`, and `# prose: keep`."
---

# Suppression Directives

A directive exempts code from *Prose*'s rewrites or lints at the file, block, line, or construct scope. The [**Suppression**](/guide/suppression) chapter covers when to reach for each, and this page is the complete list.

A pragma from another tool *(`# pylint: disable`, `# type: ignore`, `# pyright: ignore`, and the rest of the Python tooling pragmas)* is invisible to *Prose* wherever it is not named below. The walker reads it as an ordinary comment and the rules ignore it, so it sits beside the directives further down with no further setup.

The foreign comments a rule does read are the ones stating an intent no static read of the file reaches, and none of them exempts anything from a rewrite or a lint. [[prune-inert-imports]] reads a trailing `# noqa`, bare or naming `F401`, and the file-level pragmas of <Tool slug="ruff" />, flake8, and pyright as marking a re-export, and [[band-constants]] reads a trailing `# noqa`, bare or naming `E402`, as pinning the statement it trails to its line, each page giving the exact forms it matches.

## Directives

<DirectiveAnatomy />

## Block Markers

`# fmt: off` and `# fmt: on` enclose a region. The `# fmt: off` line is the marker and the next line is the first exempt line, and `# fmt: on` turns formatting back on from the line after it.

<Fixture rule="suppression" case="fmt_block_survives_between_runs" />

`# prose: off` and `# prose: on` work identically and share the same machinery, so a project can use whichever prefix reads better. `# yapf: disable` and `# yapf: enable` are block markers too, for a project moving from yapf.

<FixtureCards rule="suppression" cases="prose_block_mirrors_fmt_span yapf_disable_enable_suppresses_span" />

Each `# fmt: off` alone on its line opens a region, and the first `# fmt: on` after it closes it. Two regions stay separate spans, a second `# fmt: off` inside a region adds nothing, and a `# fmt: on` with no region open does nothing. A `# fmt: off` trailing a statement is an ordinary comment rather than a marker. Every rule leaves the inside of a region alone, the rules that rewrite a comment included.

<FixtureCards rule="suppression" cases="two_blocks_stay_separate consecutive_off_flattens bare_on_without_off_noop trailing_off_not_block_opener off_block_keeps_trailing_comma align_comments_respects_an_off_region normalize_comment_spacing_respects_an_off_region" />

## Line Markers

Line-level directives come in two families, `skip` for rewrites and `ignore` for lints, and the two are independent so a line can carry one of each.

### Rewrite Suppression

`# fmt: skip` *(or its equivalent `# prose: skip`)* exempts the logical line it ends from every rewrite.

<Fixture rule="suppression" case="fmt_skip_pins_one_member" />

<FixtureCards rule="suppression" cases="prose_skip_pins_one_member" />

A directive at the end of a wrapped statement covers every physical line the statement spans, from its first line through the directive's own, so a rule's edits to that statement are withheld together rather than applied to part of it.

<Fixture rule="suppression" case="skip_pins_a_wrapped_assignment" />

<FixtureCards rule="suppression" cases="rule_skip_pins_a_wrapped_assignment skip_pins_a_wrapped_collection skip_pins_a_wrapped_signature skip_pins_a_wrapped_if_header" />

A directive inside a bracketed construct, where the logical line continues past it, covers only its own physical line.

The bracketed forms of `# prose: skip` narrow the exemption to the listed rules, and every other rule still rewrites the line.

<Fixture rule="suppression" case="skip_one_rule_others_compose" />

<FixtureCards rule="suppression" cases="skip_per_rule_neighbor_still_strips" />

An unknown slug in the list holds nothing out, and [[inert-directives]] reports it.

<FixtureCards rule="suppression" cases="retired_slug_skip_pins_nothing" />

A skipped single-line statement inside an alignment group stays as written while the rows around it still align on one column, so the exempt row reads as a deliberate exception rather than breaking the group, whereas a skipped statement spanning several lines ends the run. A skip naming a layout rule (*`# prose: skip[reflow-calls]`, `# prose: skip[reflow-collections]`*) leaves the line that rule would have split running past its cap, and [[line-overflow]] reports that line unless a `# prose: ignore[line-overflow]` on it silences the report too. Every other rule measures the skipped line where the skip leaves it.

<FixtureCards rule="suppression" cases="chain_inside_a_skipped_call_measured_where_it_sits" />

A skip reaches the rewriting rules only, so a lint finding on the statement still prints, its message closing on the `# prose: ignore[<rule>]` that silences it. That directive sits on the line the finding starts on, which for a wrapped statement is its first line rather than the line carrying the skip. The block markers above differ, since a `# fmt: off` region suppresses rewrites and lint findings together.

<FixtureCards rule="suppression" cases="skip_leaves_lint_diagnostics_alone" />

A skipped statement and every statement inside an exempt region hold their slots when a rule reorders the body around them. [[alphabetize-siblings]] sorts the classes and functions around a skipped statement past it and the imports above and below it as two separate runs. A region's `# prose: off` and `# prose: on` lines stay where they stand and divide the body, so the siblings on each side of a region sort as runs of their own. [[group-imports]] groups the imports on either side of a skipped statement or a region apart, and the rewrites inside a class or function holding an exempt line still reach every other line of it.

### Lint Suppression

`# prose: ignore` and its bracketed forms silence lint findings on the same line, and the rewrites still reach it.

<Fixture rule="suppression" case="ignore_suppresses_one_lint_line" />

A bare `# prose: ignore` silences every lint on the line, and a bracketed list only the rules it names. [[inert-directives]] reports an `ignore` that silences nothing, and a bare one that silences findings, so it can be narrowed to the slugs it silences.

## Construct Order Preservation

`# prose: keep` holds one construct in the order written, and which construct it covers depends on where it sits. On the opening `{` line or the closing `}` line of a dict literal, it keeps the entry order, so [[alphabetize-siblings]] leaves the entries alone.

<Fixture rule="alphabetize_siblings" case="dict_keep_marker_vs_sorted" />

The directive covers that one literal, where [[alphabetize-siblings]] keeps the entry order and [[band-constants]] leaves the statement out of the band, and the same marker on an `__all__` or `__slots__` list keeps that one list as written.

<FixtureCards rule="alphabetize_siblings" cases="dunder_list_keep_marker_holds_order" />

On the `class` line, or on the line holding a wrapped header's closing `:`, the marker keeps the statements of the class body in the order written. That fits a class whose field order means something to the library reading it, such as a `pandera.DataFrameModel`, whose columns follow the order its fields are declared in.

<Fixture rule="alphabetize_siblings" case="class_keep_marker_vs_sorted" />

<FixtureCards rule="alphabetize_siblings" cases="class_keep_marker_trails_the_header" />

At the level of the class body, the marker holds every order a rule would otherwise change:

- [[alphabetize-siblings]] and [[group-imports]] leave the statements of the body in the order written, including those inside an `if`, a `try`, or a `with` at that level, since none of them opens a scope of its own
- [[reflow-imports]] merges the members of one module only across consecutive statements
- The entries under each heading of the class docstring that name a field follow the order of the fields, and an entry naming no field keeps its written slot among them
- [[unsorted-positionals]] passes over the field run

<FixtureCards rule="alphabetize_siblings" cases="class_keep_marker_mirrors_docstring_entries class_keep_marker_leaves_raises_as_written class_keep_marker_collapses_an_import_gap" />

Every expression and every nested scope inside the body still sorts, and so does the class itself among its siblings, because the marker holds the body rather than the class's place:

- A dict in a field's default and the keywords of a call
- The names one import statement binds
- The items of a `__slots__` or `__all__` list, which take a `# prose: keep` of their own on the list's bracket line
- The body of a method
- The body of a nested class carrying no marker of its own

<FixtureCards rule="alphabetize_siblings" cases="class_keep_marker_leaves_nested_sorts class_keep_marker_on_a_nested_class" />

## Composition

One line can carry a block marker, `# fmt: skip` or `# prose: skip` directives, `# prose: ignore[...]` directives, and `# prose: keep` together. *Prose* parses each on its own, so they combine in any order. A bare `# prose: ignore` *(no bracket list)* widens any `# prose: ignore[<rule>]` on the same line so every lint on the line is silenced, and a bare `# prose: skip` or a `# fmt: skip` widens a bracketed `# prose: skip[<rule>]` the same way for rewrites. Two bracketed directives of the same family on one line combine their slugs, so `# prose: ignore[reassigned-constants]  # prose: ignore[inlinable-bindings]` silences both lints.

<Fixture rule="suppression" case="skip_and_ignore_same_line" />

<FixtureCards rule="suppression" cases="fmt_skip_widens_a_listed_skip skip_redundant_inside_off_block all_directives_preformatted_noop" />

::: warning Malformed Directives No-Op
A malformed directive *(unclosed brackets, a misspelled keyword, trailing text after `ignore`)* is read as no directive, so it reports nothing and rewrites nothing.
:::

## File-Level Suppression

`# prose: off` on a comment line of its own *(not at the end of a statement)* opens an exempt region at that line. With no `# prose: on` after it, the region runs to the end of the file, exempting every line below the marker from every rewriting rule and every lint, whereas inside a notebook it closes at the end of its own code cell rather than reaching the next. At the top of a file the directive therefore covers the whole file, and `# fmt: off` there does the same.

<Fixture rule="suppression" case="prose_off_top_suppresses_file" />

<FixtureCards rule="suppression" cases="fmt_off_top_suppresses_file unmatched_off_suppresses_to_eof" />

A `# prose: off` at the end of a statement *(such as `x = 1  # prose: off`)* is ignored, since the directive opens a region only from a comment line of its own. The file-level form is the widest scope, and a bounded region takes the block markers above.

## Composition With `--select` / `--ignore`

Line and block directives apply on top of the active rule set. `--select align-equals` narrows the run to one rule, and `# prose: skip[align-equals]` still exempts its line from that rule. `--ignore reassigned-constants` removes a rule from the run, so a line carrying `# prose: ignore[reassigned-constants]` changes nothing, since the rule was not going to report, and [[inert-directives]] spares the directive rather than reporting it.

The [**Configuration**](/reference/configuration) reference covers the per-rule `enabled` facet.
