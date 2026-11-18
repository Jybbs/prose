---
caption : "Rewrites a docstring's quotes to the triple-double-quote form and puts a multi-line docstring's opener and closer on their own lines."
related : [wrap-docstrings, expand-docstrings]
layout  : doc
---

# frame-docstrings

<RuleLayout rule="frame_docstrings">

`frame-docstrings` rewrites every docstring to the `"""` form, because Python takes any string literal standing first in a module, class, or function as its docstring whatever quotes surround it, and the quotes are the docstring's frame. A `'''`-delimited docstring, a plain `'...'` or `"..."`, and an already-`"""` docstring all end up on the same triple-double-quote delimiter, with a raw `r` prefix kept as written on the opener since PEP 257 sanctions `r"""` for a docstring carrying a backslash.

For a multi-line docstring the rule also puts the opening `"""` on its own line at the docstring's indent and drops the closing `"""` to its own line beneath the last content line, leaving the prose body between them as written.

A string the [[docstring]] walker skips stays as written, as the [**Docs Rules**](/rules/docs/) page lists. The rule keeps the original quotes rather than corrupt the string where re-delimiting to `"""` would break it, because the body already carries a `"""` run or a single-line body ends in `"`, and a docstring whose body is blank keeps its quotes too.

</RuleLayout>
