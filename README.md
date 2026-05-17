<div align="center">
<img src="assets/brand/title.svg" alt="Prose" width="800">
<h3><em>A Python typesetter for the reader</em></h3>

[![Rust](assets/badges/rust.svg)![1.82+](https://img.shields.io/badge/1.82+-8a80cb?style=for-the-badge)](https://www.rust-lang.org/)
[![Python](assets/badges/python.svg)![3.10+](https://img.shields.io/badge/3.10+-8a80cb?style=for-the-badge)](https://www.python.org/)
[![Coverage](assets/badges/coverage.svg)![percent](https://img.shields.io/codecov/c/github/Jybbs/prose?style=for-the-badge&label=&color=8a80cb)](https://codecov.io/gh/Jybbs/prose)

</div>

---

## 🪻 About

***Prose*** formats Python source to be *legible at a glance*. It aligns equals signs and colons vertically across consecutive lines, places one entry per line in dictionaries and lists, alphabetizes methods and fields within their groups, applies a singleton rule for colon padding, and treats code like prose rather than minified text.

> [!NOTE]
> ***Prose*** is still pre-1.0. `0.2.0` lifts the catalog to **eighteen** [rules](#-rules) (*fourteen auto-fix, four lint-only*) and lands the surrounding surface (*structured output formats, suppression directives, rule subsetting, an exit-code matrix*), with additional rules and configuration knobs planned for later releases.

---

## 🗞️ Philosophy

Code is read far more often than it is written. A reader's eye moves down a page and across adjacent lines looking for parallels, patterns, and **shape**. When every `=` sits at a different column and every collection is compressed onto one line, that shape disappears, forcing the eye to slow down. ***Prose*** restores it, with aligned columns letting the eye skim, one-per-line collections making each entry a unit, and alphabetized groupings giving every reader the same landmarks.

The trade-offs minimalist formatters were built to avoid (*wider diffs, more vertical scrolling, occasional re-alignment churn*) no longer dominate the equation. Agentic assistants do much of the typing, and every modern code host offers whitespace-ignoring diffs. What remains is the daily experience of reading code.

---

## 🪄 Install & Usage

```bash
uv tool install prose-formatter
```

The binary exposes `format`, `check`, and `completions`:

```bash
prose format path/                          # rewrite files in place
prose check path/                           # exit non-zero on violations
prose format --diff path/                   # show the diff without writing
prose check --stdin < file.py               # read from stdin
```

Subset the active rules with `--select` and `--ignore`:

```bash
prose check --select align-equals path/         # run a single rule
prose check --ignore strip-trailing-commas path/  # subtract one
prose check --select align-equals,align-colons path/  # comma list
```

Pick a structured output format for editors, pre-commit, and CI:

```bash
prose check --output-format json path/      # newline-delimited JSON
prose check --output-format github path/    # GitHub Actions annotations
prose check --output-format sarif path/     # SARIF for Code Scanning
prose check --color always path/            # force color, NO_COLOR honored
prose completions zsh                       # shell completion script
```

Source ranges may opt out of formatting via block markers:

```python
# fmt: off
keep_this_block_exactly_as_written = (1,2,3)
# fmt: on
```

`# fmt: skip` on the same line opts out a single statement, and `# yapf: disable` / `# yapf: enable` are honored as block-level aliases. Lint diagnostics opt out per line through `# prose: ignore[<rule>]`. A bare `# prose: ignore` suppresses every lint rule on the line, and `# prose: ignore[a, b]` lists several.

---

## 🦉 Exit Codes

The binary resolves every run into one of **five** exit codes, which CI gates and pre-commit hooks compile against:

| Code | Meaning |
|---|---|
| `0` | **Clean**: no diagnostics, no rewrites pending |
| `1` | **Format would change**: at least one auto-fix diagnostic |
| `2` | **Lint violation**: at least one lint-only diagnostic |
| `3` | **Parse error**: input could not be parsed as Python |
| `4` | **Config error**: `pyproject.toml`, `--select` / `--ignore`, or argument validation |

When two outcomes apply to the same run, the higher number wins. `prose --help` prints the same table beneath the option list. In `format` mode, code `1` is suppressed when the rewrite succeeds because the changes were applied rather than left pending. Codes `2`, `3`, and `4` apply identically across both subcommands.

---

## 🪶 Rules

Auto-fix rules:

| Rule | Coverage |
|---|---|
| `align-colons` | Collection literals, Pydantic / dataclass fields, function signatures, and docstring `Args:` sections |
| `align-equals` | Consecutive assignments at the same indentation |
| `align-imports` | The `import` keyword in `from ... import ...` groups and `as` in `import ... as ...` groups |
| `alphabetize` | Classes, methods (*grouped dunders → properties → privates → publics*), enum members, Pydantic fields (*required then optional*), function parameters, keyword arguments, and `from` imports |
| `bare-import-allowlist` | Bare `import X` outside a configurable allowlist (*default `numpy`, `pandas`*) |
| `blank-lines` | Module-level `def` and `class` carry 2 blank lines before them, methods inside a class body carry 1, a module-level statement after `if __name__ == "__main__":` carries 1, and adjacent module-level bare `import` and `from` imports carry 1 between them |
| `collection-layout` | Expands `dict`, `list`, and `set` literals to one entry per line, even when they fit inline |
| `docstring-wrap` | Wraps docstring description prose to `docstring-line-length` and structured `Args:` / `Returns:` / `Raises:` sections to the budget chosen by `docstring-structured-policy` |
| `match-case-align` | Single-expression case bodies |
| `multi-line-docstrings` | Multi-line docstring placement, with opener and closer each on their own line |
| `no-single-line-docstrings` | Single-line triple-quoted docstrings, expanded into the canonical multi-line shape |
| `singleton-rule` | Skips colon padding when only one item exists in the aligned group |
| `strip-trailing-commas` | Multi-line collections and signatures |
| `unused-future-annotations` | Removes `from __future__ import annotations` when removal is provably safe for the file |

Lint-only rules:

| Rule | Coverage |
|---|---|
| `legacy-union-syntax` | `Union[X, Y]` and `Optional[X]` when `target-version` is 3.10 or higher, with the PEP 604 `X \| Y` / `X \| None` shapes as the recommendation |
| `loose-constants` | Module-level `SCREAMING_CASE` assignments that would read better as an `Enum`, a model field, or a function-local |
| `no-step-narration` | Own-line numbered-step comments (*`# 1. ...`, `# Step 2: ...`*) that signal extractable functions |
| `single-use-variables` | Bindings assigned and read exactly once, where inlining the right-hand side carries the same meaning |

### Example

Before:

```python
from pydantic import BaseModel
from datetime import date
from collections.abc import Iterable
from decimal import Decimal

DEFAULT_LIMIT = 50
RETRY_INTERVAL = 30
PAGE_SIZE = 25

class Posting(BaseModel, extra="forbid"):
    title: str
    company: str
    skills: Iterable[str] | None = None
    location: str | None = None
    date_posted: date
    salary_max: Decimal | None = None

    def render(self, separator: str, include_location: bool, include_date: bool, max_width: int = 80) -> str: ...

config = {"threshold": 0.7, "metric": "euclidean", "linkage": "ward", "n_clusters": None, "random_state": 42,}
```

After:

```python
from collections.abc import Iterable
from datetime        import date
from decimal         import Decimal
from pydantic        import BaseModel

DEFAULT_LIMIT  = 50
RETRY_INTERVAL = 30
PAGE_SIZE      = 25

class Posting(BaseModel, extra="forbid"):
    company     : str
    date_posted : date
    title       : str
    location    : str | None           = None
    salary_max  : Decimal | None       = None
    skills      : Iterable[str] | None = None

    def render(
        self,
        include_date     : bool,
        include_location : bool,
        separator        : str,
        max_width        : int = 80
    ) -> str: ...

config = {
    "linkage"      : "ward",
    "metric"       : "euclidean",
    "n_clusters"   : None,
    "random_state" : 42,
    "threshold"    : 0.7
}
```

---

## ⚖️ Configuration

***Prose*** loads the nearest `[tool.prose]` section found by walking upward from the working directory. With no configuration, every rule runs at its default. To tune a rule, write its sub-table:

```toml
[tool.prose]
code-line-length = 88

[tool.prose.rules.align-equals]
enabled = false                # disable a rule outright

[tool.prose.rules.align-colons]
max-shift        = 12          # cap padding width
max-shift-policy = "drop"      # how to handle overrun groups

[tool.prose.rules.collection-layout]
max-atomics-per-line = 3       # keep short tuples on one line
```

Per-rule knobs:

| Key | Type | Where | Meaning |
|---|---|---|---|
| `enabled` | `bool` | every rule sub-table | Toggle the rule on or off, defaulting to `true` |
| `max-shift` | positive int | alignment rules | Ceiling on per-line padding, defaulting to `8` |
| `max-shift-policy` | `"split"` \| `"drop"` \| `"skip"` | alignment rules | How to handle a group whose widest member exceeds `max-shift`. `split` partitions the group, `drop` excludes the widest members from the padding calculation, `skip` leaves the whole group unaligned, defaulting to `"split"` |
| `max-atomics-per-line` | positive int | `collection-layout` | Keep short collections on one line when each entry is an atomic literal and the run fits the cap, defaulting to `8` |
| `allow` | list of module names | `bare-import-allowlist` | Modules whose bare-import form is preserved, defaulting to `["numpy", "pandas"]` |
| `allow` | list of names | `loose-constants` | Module-level names exempted from the lint, defaulting to `[]` |
| `allow-pattern` | regex | `single-use-variables` | Binding names exempted from the lint, defaulting to `"^_"` |
| `code-line-length` | positive int | top-level `[tool.prose]` | Honored by line-length-aware rules, defaulting to `88` |
| `docstring-line-length` | positive int | top-level `[tool.prose]` | Description-prose budget for `docstring-wrap`, defaulting to `76` |
| `docstring-structured-policy` | `"code-line-length"` \| `"docstring-line-length"` | top-level `[tool.prose]` | Source budget for structured docstring sections, defaulting to `"code-line-length"` |
| `target-version` | `"3.X"` version string | top-level `[tool.prose]` | Python runtime the project ships to. Consumed by version-gated rules, defaulting to unset |

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Structured sections (*`Args:`, `Returns:`, `Raises:`*) read as code-shaped tables and reuse `code-line-length` (*88 by default*) to match surrounding indentation, though `docstring-structured-policy` switches them to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The `docstring-wrap` rule consumes both budgets.

`target-version` names the Python runtime a project ships to, taking the bare `major.minor` form (*`"3.13"`, `"3.14"`*) used by `mypy`'s `python_version` setting. Rules whose safety depends on the runtime read this field directly, treating an unset value as the cue to skip every version-dependent arm rather than assume a default. `legacy-union-syntax` is the first such consumer, staying quiet on every project that has not opted into a target.

**Alignment rules** are `align-colons`, `align-equals`, `align-imports`, and `match-case-align`. **Toggle-only rules** carry only the `enabled` knob. They are `alphabetize`, `blank-lines`, `docstring-wrap`, `legacy-union-syntax`, `multi-line-docstrings`, `no-single-line-docstrings`, `no-step-narration`, `singleton-rule`, `strip-trailing-commas`, and `unused-future-annotations`. The remaining rules (`bare-import-allowlist`, `collection-layout`, `loose-constants`, `single-use-variables`) carry rule-specific knobs documented in the table above.

Per-invocation overrides via `--select` and `--ignore` (*see [Install & Usage](#-install--usage) above*) take precedence over the configured-enabled set.

---

## 🗺️ Composition

***Prose*** runs as the second pass in a **two-stage pipeline**. The first pass owns tokens (*line wrapping, quote normalization, indentation, blank-line discipline*) and the second pass owns layout (*alignment, alphabetization, the singleton rule, one-entry-per-line collections, trailing-comma stripping*). [Ruff](https://docs.astral.sh/ruff/) is the canonical first pass, in that `ruff format` matches the token-level scope and its lint config shares the `pyproject.toml` root with `[tool.prose]`.

```bash
ruff format && prose format
```

> [!IMPORTANT]
> Running ***Prose*** first is incorrect, because ***Prose***'s alignment math depends on already-settled line breaks and an upstream re-wrap will undo per-line layout decisions, forcing a third pass.

### Ruff Configuration

| Code | Conflict | Reason |
|---|---|---|
| `COM812` | Lint re-adds trailing commas | `strip-trailing-commas` removes them in multi-line collections and signatures |
| `E203` | Lint flags whitespace before `:` | `align-colons` produces it in dict literals, dataclass fields, function signatures, and docstring `Args:` blocks |
| `E221` | Lint flags multiple spaces before `=` | `align-equals` produces it across consecutive assignments at the same indentation |
| `E272` | Lint flags multiple spaces before `import` / `as` | `align-imports` produces it across `from ... import ...` and `import ... as ...` groups |
| `E501` | Lint flags lines past `line-length` | A long member in an alignment group pads shorter lines rightward, occasionally past the configured limit |
| `skip-magic-trailing-comma` | Formatter re-expands collections by trailing-comma presence | `prose format` controls collection layout independently of comma signaling |

### Other Tools

Black formats, Flake8 lints, and isort sorts, so each pairs with ***Prose*** at a different layer:

| Tool | Pairing |
|---|---|
| **[Black](https://black.readthedocs.io/)** | Run Black with `--skip-magic-trailing-comma`, then ***Prose*** second. Black collapses collections that `collection-layout` re-expands and preserves trailing commas that `strip-trailing-commas` removes |
| **[Flake8](https://flake8.pycqa.org/)** | Add `extend-ignore = E203, E221, E272` to `.flake8` or `setup.cfg` (*and `C812` if [`flake8-commas`](https://github.com/PyCQA/flake8-commas) is installed*). Flake8 inherits the same `pycodestyle` codes Ruff inherits |
| **[isort](https://pycqa.github.io/isort/)** | Run isort first, ***Prose*** second, with no configuration adjustment. ***Prose*** alphabetizes within isort's groups and aligns the `import` keyword that isort leaves un-aligned |

---

## 🪡 Integrations

Wire ***Prose*** into anything that runs on save, on commit, or in CI.

### GitHub Actions

The minimal check-on-CI shape:

```yaml
- run: uv tool install prose-formatter
- run: prose check .
```

For inline annotations on the PR diff, use the `github` output format. ***Prose*** emits [workflow commands](https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions) that GitHub renders as native check-run annotations:

```yaml
- run: uv tool install prose-formatter
- run: prose check --output-format github .
```

For findings that persist across runs and surface in [Code Scanning](https://docs.github.com/en/code-security/code-scanning), emit SARIF and upload it:

```yaml
- run: uv tool install prose-formatter
- run: prose check --output-format sarif . > prose.sarif
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: prose.sarif
```

CI gates compile against the [exit-code matrix](#-exit-codes) above. A non-zero status without `continue-on-error` fails the step.

### Editor

Any editor that supports run-on-save (*VSCode's `runOnSave`, Vim's `autocmd BufWritePost`, JetBrains File Watchers*) can shell out to `prose format <file>`. For editors that consume structured diagnostics, `prose check --output-format json --stdin` emits one [Ruff-shaped](https://docs.astral.sh/ruff/configuration/#output-format) record per line.

### Pre-Commit

Add a `local` hook to your `.pre-commit-config.yaml`:

```yaml
- repo: local
  hooks:
    - id: prose
      name: prose
      entry: prose format
      language: system
      types: [python]
```

Swap `entry: prose format` for `entry: prose check` for the check-only variant. The hook surfaces the same exit codes the CLI uses, so a `format` hook never fails on rewrites it applies and a `check` hook fails the commit when changes are pending.

### Shell Completions

```bash
prose completions zsh > "${fpath[1]}/_prose"
prose completions bash > /etc/bash_completion.d/prose
prose completions fish > ~/.config/fish/completions/prose.fish
```

Both `elvish` and `powershell` are supported targets for the `completions` subcommand.

---

## 🗜️ Development

### One-Time Setup

***Prose*** uses [mise](https://mise.jdx.dev) to manage every toolchain and CLI through a single `mise.toml`. Install mise, wire it into your shell, then `mise install` provisions the rest.

#### Install Mise

```bash
curl https://mise.run | sh
```

The installer drops the binary into `~/.local/bin/mise`.

#### Wire Mise into Zsh

Three init files cover the three load contexts (*every shell, login shells, interactive shells*) so mise-managed tools resolve correctly whether you are inside an interactive terminal, a login shell, or a non-interactive subprocess:

```bash
# ~/.zshenv  (sourced for every shell, including non-interactive)
export PATH="$HOME/.local/bin:$PATH"

# ~/.zprofile  (sourced for login shells, before .zshrc)
eval "$(mise activate zsh --shims)"

# ~/.zshrc  (sourced for interactive shells)
eval "$(mise activate zsh)"
```

`.zshenv` puts `mise` itself on `PATH` so the later `eval` lines can find it. `.zprofile`'s `--shims` activation makes mise-managed binaries resolvable in non-interactive contexts (*scripts, editors, GUI launches*). `.zshrc`'s full activation gives interactive shells the per-directory tool resolution and task discovery.

#### Clone and Provision

```bash
git clone https://github.com/Jybbs/prose.git
cd prose
mise install
```

`mise install` provisions the base toolchain:

| Tool | Purpose |
|---|---|
| `maturin` | Rust → Python wheel builder |
| `python` (3.14) | Python interpreter for wheel builds |
| `rust` (stable) | Rust toolchain via rustup |
| `uv` | Python package and venv manager |

Additional task-scoped tools (*`cargo-insta`, `cargo-machete`, `cargo-llvm-cov`, `codecov-cli`*) install on demand when their owning task first runs, so no second provisioning step is required.

### Tasks

Tasks live in `mise.toml`. GitHub Actions drives most of them across the CI workflows that gate every push and pull request, so what passes locally with `mise ci` mirrors what runs upstream. The same tasks are available locally for reproducing a failed CI step, accepting snapshot updates after a fixture change, or running an individual check during iteration. List them with `mise tasks`.

| Command | What it does |
|---|---|
| `mise audit` | Detect unused dependencies via `cargo machete` |
| `mise build` | Compile in debug mode |
| `mise check` | Verify Rust source matches `rustfmt` without rewriting |
| `mise ci` | Full local sweep: `audit`, `check`, `lint`, `test`, `wheel` |
| `mise coverage` | Generate an LCOV coverage report at `target/lcov.info` |
| `mise format` | Format Rust source with `rustfmt` |
| `mise lint` | Run `clippy` with all warnings as errors |
| `mise readme` | Rewrite `README.md` in place to absolute URLs for PyPI |
| `mise review` | Interactively review pending snapshot diffs |
| `mise test` | Run all tests including `insta` snapshots |
| `mise upload` | Generate the coverage report and upload it to Codecov |
| `mise wheel` | Build the `maturin` wheel into the active virtualenv |

A common iteration flow on a rule or fixture is to edit, run `mise test`, accept any snapshot diffs with `mise review`, then run `mise ci` before pushing.
