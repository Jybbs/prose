---
title: Installation
---

*Prose* ships as a single native binary written in Rust, distributed as a Python wheel so the install path lands on PyPI with no separate toolchain. The binary runs on Linux, macOS, and Windows, with no Python interpreter on the hot path. The recommended install is through , in that `uv tool install` fetches the platform wheel and exposes the `prose` executable on the user's `PATH` without an explicit venv.
