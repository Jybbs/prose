---
caption : "Settles quote character, string prefix, and numeric case on one spelling per literal."
related : [docstring-frame, miscased-constants, line-overflow]
layout  : doc
---

# normalize-literals

<RuleLayout rule="normalize_literals">

A value written three ways reads as three values. `0XABC` beside `0xabc`, `U"y"` beside `'y'`, and `1E5` beside `1e5` each name one thing while asking the reader to reconcile how it was typed before comparing it to anything. `normalize-literals` settles every literal on one spelling so the eye compares values rather than typography, and because the three normalizations share a single walk of the token stream, they land as facets of one rule rather than three rules firing over the same tokens.

The `unify-quotes` facet settles a string on `"`, falling back to `'` only where that drops an escape, so `'plain'` becomes `"plain"` while `'say "hi"'` keeps the quotes that spare it two backslashes. A quote counts once wherever it appears, escaped or not, so a body spelling `\"` inside single quotes sheds a backslash it never needed and the delimiter stays put. A raw string never gains or loses a backslash, so it swaps only when every `"` it holds already carries one, and a triple-quoted string swaps only when no `"""` run and no trailing `"` would abut the closer. The facet passes over the docstring slot, whose quotes [[docstring-frame]] canonicalizes to the `"""` frame, and over any literal inside a replacement field, whose quotes the enclosing f-string constrains before Python 3.12. The slot is read by position rather than by part count, so an implicitly concatenated leading expression holds its quotes too.

Under `unify-prefixes` every prefix letter goes lowercase and the no-op `u` goes entirely, so `U"y"` reads `"y"`, `F"{x}"` reads `f"{x}"`, and `BR"z\d"` reads `rb"z\d"` with the letters ordered raw-first. `unify-numerics` then reaches the numeric spelling, uppercasing hex digits while lowercasing the `0x`, `0o`, and `0b` radix markers, the `e` exponent, and the `j` suffix, which leaves `0XdeadBEEF` as `0xDEADBEEF` and `10E+3J` as `10e+3j`. The digits, the `_` separators, and every escape sequence that is not a quote pass through exactly as written.

The rule opens the pipeline, so every length-aware rule downstream measures a literal at the width it will ship at rather than the width it was typed at.

<template #configuration>

<RuleConfigTable />

Each facet gates one spelling axis independently, so a project that has settled its quotes by hand can run `unify-quotes = false` while the prefix and numeric spellings still normalize. Setting `enabled = false` turns all three off together.

</template>

<template #related-after>

For a single literal that must keep the spelling it was written with, [**Suppression**](/usage/suppression) covers the `# prose: ignore[normalize-literals]` line directive and the `# fmt: off` / `# fmt: on` block markers.

</template>

</RuleLayout>
