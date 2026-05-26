# Installation

*Prose* ships as a single native binary written in Rust, distributed as a Python wheel so the install path lands on PyPI with no separate toolchain. The binary runs on Linux, macOS, and Windows, with no Python interpreter on the hot path. The recommended install is through <Tool slug="uv" />, in that `uv tool install` fetches the platform wheel and exposes the `prose` executable on the user's `PATH` without an explicit venv.

## Install

```bash
uv tool install prose-formatter
```

Two names are worth flagging up front. The PyPI distribution is `prose-formatter`, because the unqualified `prose` name was already claimed when the project shipped. The binary the wheel installs is `prose`, because that's the name a user types at the shell and a CI step writes into a workflow. Every later command in the documentation assumes the `prose` binary on `PATH`.

`pip install prose-formatter` and `pipx install prose-formatter` work the same way for users who prefer those package managers. The PyPI distribution is the same wheel in every case, so the install path is whatever fits the project's existing tooling.

Confirm the install with:

```bash
prose --version
```

## Platforms

Pre-built wheels cover the following targets:

| Triple | Platform |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux x86_64 *(glibc, manylinux)* |
| `aarch64-unknown-linux-gnu` | Linux aarch64 *(glibc, manylinux)* |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x86_64 |

A source distribution rides alongside the wheels for any target outside this matrix *(musl-based Linux distros, FreeBSD, 32-bit architectures)*. Installing the sdist requires a Rust toolchain on the install host, because the installer builds the binary from source rather than fetching a pre-built artifact.

## Python Compatibility

The install path needs Python **3.10 or newer**, which is the lower bound declared in the wheel's `requires-python` metadata. The Python interpreter is used only by the installer *(uv, pip, pipx)* to land the binary on `PATH`, and the running formatter doesn't load it on the hot path. For the runtime version a project's source itself targets *(read by [[legacy-union-syntax]] and [[unused-future-annotations]] when judging safety)*, see the `target-version` field in the [**Configuration**](/reference/configuration) reference.

## Next Steps

The [**Quick Start**](/usage/quick-start) chapter walks through the first `prose format` and `prose check` invocations. The [**Ruff**](/integrations/ruff) integration page covers the `ruff format && prose format` recipe for projects that pair the two. The [**Configuration**](/reference/configuration) reference enumerates every per-rule knob. For shell completions, see [**Shell Completions**](/integrations/shell-completions) under Integrations.
