---
title: Editor
---

*Prose* reaches an editor through a language server or a shellout. The **`prose server`** language server is the richest, giving format-on-save and live diagnostics over the protocol an editor already speaks. For editors without a language-server client, `prose format ` rewrites on save and `prose check --output-format json --stdin` emits one **Ruff-shaped** diagnostic record per line. The shellout paths read from disk or stdin, where the server tracks the editor's live buffer directly.
