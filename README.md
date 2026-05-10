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
> ***Prose*** is currently in alpha, meaning the **eight** [auto-fix rules](#rules) below ship in `0.1.x`. The `0.2` cycle expands the surface around them (*structured output formats, suppression directives, rule subsetting, an exit-code matrix*) and brings additional auto-fix and lint-only rules online.

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

`# fmt: skip` on the same line opts out a single statement. Line-level lint suppression via `# prose: ignore[<rule>]` lands alongside the lint rules in `0.2`.

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

| Rule | Coverage |
|---|---|
| `align-colons` | Collection literals, Pydantic / dataclass fields, function signatures, and docstring `Args:` sections |
| `align-equals` | Consecutive assignments at the same indentation |
| `align-imports` | The `import` keyword in `from ... import ...` groups and `as` in `import ... as ...` groups |
| `alphabetize` | Classes, methods (*grouped dunders → properties → privates → publics*), enum members, Pydantic fields (*required then optional*), function parameters, keyword arguments, and `from` imports |
| `collection-layout` | Expands `dict`, `list`, and `set` literals to one entry per line, even when they fit inline |
| `match-case-align` | Single-expression case bodies |
| `singleton-rule` | Skips colon padding when only one item exists in the aligned group |
| `strip-trailing-commas` | Multi-line collections and signatures |

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
| `code-line-length` | positive int | top-level `[tool.prose]` | Honored by line-length-aware rules, defaulting to `88` |
| `docstring-line-length` | positive int | top-level `[tool.prose]` | Description-prose budget for `docstring-wrap`, defaulting to `76` |
| `docstring-structured-policy` | `"code-line-length"` \| `"docstring-line-length"` | top-level `[tool.prose]` | Source budget for structured docstring sections, defaulting to `"code-line-length"` |

Docstrings carry two readings inside one triple-quoted region. Description prose between the opening `"""` and the first section heading reads as paragraphs, where 76 characters is the comfortable line for sustained reading. Structured sections (*`Args:`, `Returns:`, `Raises:`*) read as code-shaped tables and reuse `code-line-length` (*88 by default*) to match surrounding indentation, though `docstring-structured-policy` switches them to `docstring-line-length` if a project prefers a single narrower budget across the whole docstring. The `docstring-wrap` rule will consume both budgets when it lands later in `0.2`.

**Alignment rules** are `align-colons`, `align-equals`, `align-imports`, and `match-case-align`. **Toggle-only rules** are `alphabetize`, `singleton-rule`, and `strip-trailing-commas`.

Per-invocation overrides via `--select` and `--ignore` (*see [Install & Usage](#install--usage) above*) take precedence over the configured-enabled set.

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

CI gates compile against the [exit-code matrix](#exit-codes) above. A non-zero status without `continue-on-error` fails the step.

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

`mise install` provisions:

| Tool | Purpose |
|---|---|
| `cargo-insta` | Snapshot test review |
| `maturin` | Rust → Python wheel builder |
| `python` (3.14) | Python interpreter for wheel builds |
| `rust` (stable) | Rust toolchain via rustup |
| `uv` | Python package and venv manager |

### Daily Workflow

Tasks are defined in `mise.toml` and discoverable via `mise tasks`:

| Command | What it does |
|---|---|
| `mise build` | Compile in debug mode |
| `mise check` | Verify Rust source matches `rustfmt` without rewriting |
| `mise ci` | Lint + test + wheel (full local sweep) |
| `mise format` | Format Rust source with `rustfmt` |
| `mise lint` | Run `clippy` with all warnings as errors |
| `mise review` | Interactively accept pending snapshot diffs |
| `mise test` | Run all tests including `insta` snapshots |
| `mise wheel` | Build the wheel and install into `.venv` |

### Editor

<details>
<summary>VSCode</summary>

Install [`rust-analyzer`](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) and [`Even Better TOML`](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml). The `rust-analyzer` extension bundles its own language server, so it works without additional global Rust installs.

Suggested user settings (*apply to any Rust project*):

```json
"rust-analyzer.check.command": "clippy",
"rust-analyzer.imports.granularity.group": "module"
```

</details>
