---
title: Pipeline Order
---

*Prose* runs each enabled rule in a deterministic order, reparsing the source between rules so every downstream rule reads a settled AST. The reparse is the discipline that makes the rule set composable, wherein no rule observes the half-applied state of another, leaving every pass free of cross-rule edit conflict by construction. The order itself is canonical, source-of-truth in `crate/src/rule.rs` *(the `register_rules!` macro block)*, and pedagogically valuable. A rule that depends on a settled token surface sits downstream of every rule that touches that surface, in that *(for example)* `align-colons` runs before `docstring-wrap` because the docstring wrap budget depends on the post-colon column the alignment rule sets.
