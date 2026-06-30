---
title: Output Formats
---

`--output-format` selects the diagnostic shape *Prose* emits, with named formats covering the common consumers. `text` is the human-readable default, rendering rustc-style snippets with carets and fix suggestions. `json` emits Ruff-shaped NDJSON for editor plugins and tooling, wherein the record shape mirrors what LSP-style diagnostic surfaces already consume. `github` emits workflow commands that  renders as inline annotations. `sarif` emits a **SARIF 2.1.0** run document for upload into **GitHub Code Scanning**, persisting findings across runs in the repository's Security tab.
