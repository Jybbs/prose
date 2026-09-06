---
description: "Covers the package managers, post-install verification, and the platforms wheels are built for."
---

# Installation

*Prose* is a single native binary, written in Rust and published on PyPI as a Python wheel, so installing it needs no Rust toolchain. Pre-built wheels cover Linux, macOS, and Windows, and the formatter runs with no Python interpreter on the hot path. The recommended installer is <Tool slug="uv" />, whose `uv tool install` downloads the wheel for your platform and puts the `prose` executable on your `PATH` with no virtual environment to manage.

## Install

```bash
uv tool install prose-formatter
```

The package name and the executable name differ, and both appear throughout the documentation. The PyPI distribution is `prose-formatter`, because the name `prose` was already taken on PyPI when the project first published. The executable the wheel installs is `prose`, which is what you type at a shell and write into a CI step. Every later command assumes `prose` is on your `PATH`.

`pip install prose-formatter` and `pipx install prose-formatter` install the same wheel, so use whichever package manager the project already uses.

Confirm the install with:

```bash
prose --version
```

## Platforms

Pre-built wheels cover the following targets:

<WheelPlatforms />

A source distribution is published beside the wheels for any other target *(musl-based Linux distributions, FreeBSD, 32-bit architectures)*. Installing from the source distribution needs a Rust toolchain on the machine, because the installer compiles the binary rather than downloading one.

## Python Compatibility

Installing needs Python **{{ $frontmatter.requiresPython }} or newer**, the lower bound the wheel declares in its `requires-python` metadata. Only the installer *(uv, pip, or pipx)* uses that interpreter, to place the binary on `PATH`, and the formatter itself never loads the interpreter. The Python version a project's own code targets is a separate setting, the `target-version` key in the [**Configuration**](/reference/configuration) reference, which the version-gated rules read before rewriting anything.

## Next Steps

- The [**Quick Start**](/usage/quick-start) chapter walks through the first `prose format` and `prose check` commands.
- The [**Ruff**](/integrations/ruff) integration page covers running Ruff in the same project.
- The [**Configuration**](/reference/configuration) reference lists every top-level key and every per-rule facet.
- [**Shell Completions**](/integrations/shell-completions) under Integrations installs tab completion for the `prose` command.
