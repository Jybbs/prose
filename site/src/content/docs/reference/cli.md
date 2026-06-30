---
title: CLI
---

The `prose` binary's subcommands each resolve a distinct workflow shape. `check` reports violations without modifying anything, `completions` emits a shell-completion script, `format` rewrites Python files in place, `rules` lists the registered rules in pipeline order, and `server` speaks the language-server protocol to an editor. `format` and `check` share the same path-handling, stdin, rule-filtering, and output-format surface, so a CI step that runs `prose check` and a developer that runs `prose format` see the same flag set with the same precedence.
