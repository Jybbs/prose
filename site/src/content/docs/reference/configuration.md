---
title: Configuration
---

*Prose* loads its configuration from a `prose.toml` file, a `.config/prose.toml`, or the `[tool.prose]` table of a `pyproject.toml`, walking upward from each input file's directory to the nearest one. With no configuration, every rule runs at its default, in that a project that writes no config gets the canonical *Prose* shape automatically.
