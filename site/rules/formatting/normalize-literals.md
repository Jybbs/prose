---
caption : "Rewrites every string and numeric literal to one spelling of quote character, string prefix, and numeric case."
related : [frame-docstrings, miscased-constants, line-overflow]
layout  : doc
---

# normalize-literals

<RuleLayout rule="normalize_literals">

`normalize-literals` rewrites every literal to one spelling, so `'y'` reads `"y"`, `U"y"` reads `"y"`, and `0XABC` reads `0xABC`. A value written three ways reads as three values, and the reader has to work out how each was typed before comparing it to anything, so the rule settles quotes, prefixes, and numeric case in one pass over the token stream, each behind a facet of one rule rather than three rules reading the same tokens.

The rule runs second in the pipeline, behind only [[shed-backslash-continuations]], so every length-aware rule downstream measures a literal at the width it ships at rather than the width it was typed at.

<template #configuration>

<RuleConfigTable />

</template>

<template #facets>

Each facet gates one spelling axis on its own, so a project that has settled its quotes by hand can set `unify-quotes = false` while the prefix and numeric spellings still normalize. Setting `enabled = false` turns all three off together.

### `unify-numerics`

`unify-numerics` rewrites the numeric spelling, uppercasing hex digits while lowercasing the `0x`, `0o`, and `0b` radix markers, the `e` exponent, and the `j` suffix, so `0XdeadBEEF` reads `0xDEADBEEF` and `10E+3J` reads `10e+3j`. The digits and the `_` separators pass through exactly as written.

### `unify-prefixes`

`unify-prefixes` lowercases every prefix letter and removes the no-op `u`, so `U"y"` reads `"y"`, `F"{x}"` reads `f"{x}"`, and `BR"z\d"` reads `rb"z\d"` with the letters ordered raw-first.

### `unify-quotes`

`unify-quotes` rewrites a string to `"`, keeping `'` only where the swap would add an escape, so `'plain'` becomes `"plain"` whereas `'say "hi"'` keeps the quotes that spare it two backslashes. A quote character counts once wherever it appears, escaped or not, so a body spelling `\"` inside single quotes drops the backslash it never needed while the delimiter stays, and every escape sequence that is not a quote passes through as written. A raw string never gains or loses a backslash, which limits its swap to a body where every `"` already carries one, and a triple-quoted string swaps only when no `"""` run and no trailing `"` would sit against the closer.

The facet skips the docstring slot, whose quotes [[frame-docstrings]] rewrites to the `"""` frame, and skips any literal inside a replacement field, whose quotes the enclosing f-string constrains before Python 3.12. The docstring slot is read by position rather than by part count, so an implicitly concatenated leading expression keeps its quotes too.

</template>

<template #related-after>

For a single literal that has to keep the spelling it was written with, [**Suppression**](/usage/suppression) covers the `# prose: skip[normalize-literals]` line directive and the `# fmt: off` / `# fmt: on` block markers.

</template>

</RuleLayout>
