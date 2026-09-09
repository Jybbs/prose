---
caption : "Converts printf-style `%` interpolation and `str.format()` calls to f-strings wherever both forms render the same text."
related : [normalize-literals, stack-adjacent-strings, modernize-annotations]
layout  : doc
---

# prefer-fstring

<RuleLayout rule="prefer_fstring">

`prefer-fstring` converts a `%`-formatted template or a `str.format()` call to the f-string that puts each expression inline where it renders, so `"%s=%s" % (key, value)` reads `f"{key}={value}"`. Each form runs behind its own facet, `rewrite-percent` covering the `%` operator and `rewrite-str-format` covering the method call.

Both facets read `target-version` and neither runs until it names Python **3.6** or higher, the release that added f-strings. A project with no `target-version` set keeps every template as written.

<Fixture rule="prefer_fstring" case="tuple_members_fill_each_spec" />

## Where a Template Holds

The rewrite is written only where both forms render the same text, so several kinds of template stay as written.

The `%` operator has semantics a replacement field does not reproduce, so these templates stay:

1. A bare right-hand side, because `value` may be a one-element tuple that `%` unpacks and a replacement field does not.
2. A `%d`, `%i`, or `%u`, because it truncates a float where `{:d}` raises.
3. A `%c`, because it maps an ordinal.
4. A width or precision on `%s`, since the width renders `None` where `{:8}` raises and the precision cuts the rendered text where `{:.3}` measures the value itself.

An argument no field reads keeps the whole call as written, because removing it would remove its evaluation. An argument two fields read keeps the call whenever evaluating it runs code, since the call evaluates it once where the fields would evaluate it twice.

<Fixture rule="prefer_fstring" case="a_repeated_effectful_argument_holds" />

A value the field itself cannot carry keeps the template too, covering a quote matching the delimiter the f-string opens with, a backslash, a line break, and a brace, which are the bounds every Python version accepts. A comment anywhere inside the template or the call keeps it as well, since an f-string has no place for one. A rewrite that would push its line past the budget keeps it too, because no layout rule reaches inside an f-string to wrap it back.

<template #configuration>

<RuleConfigTable />

The `target-version` field from the top-level [**Configuration**](/reference/configuration#top-level-keys) gates both facets per project, and an unset field keeps every template as written.

</template>

<template #facets>

Both facets build the same replacement field, so a conversion and a format spec pass through unchanged wherever the f-string grammar matches the template's. The printf flags translate to their format-spec counterparts, so `-` reads as `<`.

### `rewrite-percent`

`rewrite-percent` reads the template's specs in order and pairs each one with the value it renders. A tuple literal binds by position, a dict literal of identifier-shaped string keys binds by name through the parenthesized mapping key, and a lone spec also reads a literal right-hand side.

<Fixture rule="prefer_fstring" case="mapping_keys_bind_by_name" />

### `rewrite-str-format`

`rewrite-str-format` resolves each field against the call's arguments, covering the automatic numbering of an empty field, an explicit index, and a keyword name, and it keeps any attribute or index the field name spelled attached to the value inline.

<Fixture rule="prefer_fstring" case="attribute_and_index_parts_follow_the_value" />

</template>

<template #related-after>

[[modernize-annotations]] reads `target-version` the same way, rewriting a legacy `typing` spelling wherever the runtime a project ships to supports the modern one.

</template>

</RuleLayout>
