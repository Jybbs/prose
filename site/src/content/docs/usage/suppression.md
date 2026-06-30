---
title: Suppression
---

*Prose* is opinionated by design, and most projects benefit from running every rule at its default. Every codebase has its corners, though, and those corners want a way to opt out without dropping a whole rule from the pipeline. The decision is which scope the exception lives at, because *Prose* exposes suppression at four scopes *(file, block, line, dict literal)* and each one fits a different shape of exception.
