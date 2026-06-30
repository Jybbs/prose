---
title: Primitives
---

*Prose* is built from a small set of shared primitives that each carry a single responsibility. A rule reads source through source, walks the AST through one of the shared walkers, emits edit lists, and surfaces diagnostics through the pipeline. Every rule in the catalog composes from the named pieces below, so a new rule lands as a thin walker plus the per-rule decision rather than a from-scratch implementation. The padding math, the comment-attachment, and the conflict discipline live once and downstream rules consume them.
