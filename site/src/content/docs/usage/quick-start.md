---
title: Quick Start
---

Three subcommands cover every shape of run *Prose* supports. `format` rewrites files in place, `check` reports violations without modifying anything, and `completions` emits a shell-completion script. The same exit-code matrix gates both `format` and `check`, meaning a CI step and a local pre-commit hook compile against the same outcomes.
