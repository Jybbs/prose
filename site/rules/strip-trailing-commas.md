---
category: auto-fix
---

# strip-trailing-commas

A trailing comma on the last entry of a multi-line collection adds a small **visual hiccup** at every block boundary without earning its keep. Each entry already has its own line, so a new entry adds a new line of its own, which leaves the trailing comma on the previous last entry with no diff-stability win to bring. *Strip-trailing-commas* removes the trailing comma from any bracketed container that carries one, and it leaves tuples alone because Python uses the trailing comma to disambiguate single-element tuples from parenthesized expressions.

The rule walks every bracketed container (*dictionaries, lists, sets, function signatures, function calls, class bases, parenthesized argument lists*) and strips the comma after the last entry when one is present. Whether the container spans one line or many doesn't affect the strip itself, since single-line atomic collections happen not to carry trailing commas in idiomatic Python and the rule rarely fires on them. Pair with [**`collection-layout`**](/rules/collection-layout) for the multi-line expansion that brings the trailing comma into reach in the first place.

## Configuration

<ToggleConfig />

## The Canonical Case

A multi-line dict literal loses its trailing comma after the strip.

<Fixture rule="strip_trailing_commas" case="dict_literal" />

## More Examples

<Fixture rule="strip_trailing_commas" case="function_signature" title="Function Signatures Drop the Trailing Comma on the Final Arg" />

<Fixture rule="strip_trailing_commas" case="function_call" title="Function Calls Drop the Trailing Comma on the Final Arg" />

<Fixture rule="strip_trailing_commas" case="list_literal" title="Lists Follow the Same Strip as Dicts" />

<Fixture rule="strip_trailing_commas" case="class_bases" title="Class Base Lists Strip the Trailing Comma" />

<Fixture rule="strip_trailing_commas" case="parenthesized_args" title="Parenthesized Argument Lists Strip the Trailing Comma" />

<Fixture rule="strip_trailing_commas" case="nested" title="Nested Multi-Line Collections Strip Independently" />

<Fixture rule="strip_trailing_commas" case="idempotent" title="Already-Stripped Source Is Left Alone" />

## Related

The strip composes with two other rules that each shape the multi-line form.

- [**`collection-layout`**](/rules/collection-layout) expands single-line collections into the multi-line shape this rule then strips.
- [**`align-colons`**](/rules/align-colons) aligns the post-key `:` separator on the expanded form, before the strip.

For per-line opt-outs (*projects that prefer the trailing comma for diff stability even on multi-line forms*), [**`Suppression`**](/guide/suppression) covers the `# fmt: off` / `# fmt: on` block markers.
