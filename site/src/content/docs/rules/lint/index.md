---
title: Lint Rules
warmth: warm
---

Lint rules surface diagnostics without rewriting source. They run under both `prose check` and `prose format`, returning exit code 2 when any lint diagnostic fires, and they never produce an edit. Lint coincides with its domain, in that every lint rule sits in the `lint` domain and every rule in the `lint` domain is a lint. The category-versus-domain distinction collapses cleanly to one landing.
