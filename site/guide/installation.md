# Installation

*Prose* ships as a single native binary written in Rust, distributed as a Python wheel so the install path lands on PyPI with no separate toolchain. The binary runs on Linux, macOS, and Windows, with no Python interpreter on the hot path. The recommended install is through <Tool slug="uv" />, in that `uv tool install` fetches the platform wheel and exposes the `prose` executable on the user's `PATH` without an explicit venv.

## Install

```bash
uv tool install prose-formatter
```

Confirm the install with:

```bash
prose --version
```

`pip install prose-formatter` and `pipx install prose-formatter` work the same way for users who prefer those package managers. The PyPI distribution is the same wheel in every case, so the install path is whatever fits the project's existing tooling.

## Next Steps

The [**Quick Start**](/guide/quick-start) chapter walks through the first `prose format` and `prose check` invocations. The [**Two-Stage Pipeline**](/guide/two-stage-pipeline) chapter explains the canonical `ruff format && prose format` sequence. The [**Configuration**](/reference/configuration) reference enumerates every per-rule knob. For shell completions, see [**Shell Completions**](/integrations/shell-completions) under Integrations.
