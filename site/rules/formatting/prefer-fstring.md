---
caption : "Converts printf-style `%` interpolation and `str.format()` calls to f-strings wherever both forms render the same text."
related : [normalize-literals, stack-adjacent-strings, modernize-annotations]
layout  : doc
---

# prefer-fstring

<RuleLayout rule="prefer_fstring">

`prefer-fstring` moves a `%`-formatted template or a `str.format()` call onto the f-string that sets each expression inline where it renders, behind a facet apiece, `rewrite-percent` reaching the `%` operator and `rewrite-str-format` reaching the method call.

Both facets read `target-version` and both hold until it names Python **3.6** or higher, the release f-strings landed in. A project that has set no `target-version` at all holds every template.

<Fixture rule="prefer_fstring" case="tuple_members_fill_each_spec" />

## What Each Facet Reaches

`rewrite-percent` reads the template through its specs and pairs each one with the value it renders. A tuple literal binds by position, a dict literal of identifier-shaped string keys binds by name through the parenthesized mapping key, and a lone spec also reads a literal right-hand side.

<Fixture rule="prefer_fstring" case="mapping_keys_bind_by_name" />

`rewrite-str-format` resolves each field against the call's arguments, covering the automatic numbering of an empty field, an explicit index, and a keyword name, and it carries any attribute or index the field name spelled onto the value inline.

<Fixture rule="prefer_fstring" case="attribute_and_index_parts_follow_the_value" />

A conversion and a format spec both pass through unchanged, in that an f-string field reads the same grammar the template did, and the printf flags translate to their format-spec counterparts so `-` reads as `<`.

## Where a Template Holds

The rewrite lands only where both forms render the same text, so several shapes stay as written.

A bare right-hand side under `%` holds, because `value` may be holding a one-element tuple that `%` unpacks and a replacement field does not. A `%d`, `%i`, or `%u` holds, because it truncates a float where `{:d}` raises, and a `%c` holds because it maps an ordinal. A width or precision on `%s` holds, since the width renders `None` where `{:8}` raises and the precision cuts rendered text where `{:.3}` measures the value itself.

An argument no field reads holds the whole call, because dropping it would drop its evaluation, and an argument two fields read holds whenever evaluating it runs code, since the call evaluates it once where the fields would twice.

<Fixture rule="prefer_fstring" case="a_repeated_effectful_argument_holds" />

A value the field itself cannot carry holds too, covering a quote closing the delimiter the f-string opened with, a backslash, a line break, and a brace, which are the bounds every Python version accepts. A comment anywhere inside the template or the call holds it as well, since an f-string has no place for one.

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates both facets per project, and an unset field holds them both.

</template>

<template #related-after>

[[modernize-annotations]] reads `target-version` on the same axis, rewriting a legacy `typing` spelling wherever the runtime a project ships to carries the modern one.

</template>

</RuleLayout>
