---
title: Rules
---

*Prose* ships its rules across two categories. Auto-fix rules rewrite source as part of `prose format` and surface as `Severity::Format` diagnostics under `prose check`. Lint rules surface as `Severity::Lint` diagnostics in both subcommands and never rewrite.
