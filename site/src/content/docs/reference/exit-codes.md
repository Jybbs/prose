---
title: Exit Codes
---

Every `prose check` and `prose format` invocation resolves into a discrete exit code that CI gates compile against. The codes are mutually exclusive at run time, in that when two outcomes apply, the higher number wins. A `format` run that auto-fixes a rule's diagnostics returns `0` once the rewrite lands *(the diagnostic was applied, not left pending)*, whereas a `check` run on the same source returns `1`.
