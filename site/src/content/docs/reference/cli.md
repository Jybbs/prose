---
title: CLI
---

The `prose` binary's subcommands each resolve a distinct workflow shape. `format` rewrites Python files in place, `check` reports violations without modifying anything, `server` speaks the language-server protocol to an editor, and `completions` emits a shell-completion script. `format` and `check` share the same path-handling, stdin, rule-filtering, and output-format surface, so a CI step that runs `prose check` and a developer that runs `prose format` see the same flag set with the same precedence.
