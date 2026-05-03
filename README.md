<div align="center">
<img src="assets/title.svg" alt="Prose" width="800">
<h3><em>A Python typesetter for the reader</em></h3>

[![Rust 1.82+](https://img.shields.io/badge/rust-1.82+-8a80cb?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Python 3.10+](https://img.shields.io/badge/python-3.10+-8a80cb?style=for-the-badge&logo=python&logoColor=white)](https://www.python.org/)
[![maturin](https://img.shields.io/badge/built_with-maturin-8a80cb?style=for-the-badge)](https://www.maturin.rs/)

</div>

---

## 🪻 About

*Prose* formats Python source to be *legible at a glance*. It aligns equals signs and colons vertically across consecutive lines, places one entry per line in dictionaries and lists, alphabetizes methods and fields within their groups, applies a singleton rule for colon padding, and treats code like prose rather than minified text.

> [!NOTE]
> Alpha (`0.1.0`). The eight rules below are stable, with additional rules planned for later releases.

---

## 🗞️ Philosophy

Code is read far more often than it is written. A reader's eye moves down a page and across adjacent lines looking for parallels, patterns, and shape. When every `=` sits at a different column and every collection is compressed onto one line, that shape disappears. *Prose* restores it: aligned columns let the eye skim, one-per-line collections make each entry a unit, alphabetized groupings give every reader the same landmarks.

The trade-offs minimalist formatters were built to avoid (*wider diffs, more vertical scrolling, occasional re-alignment churn*) no longer dominate the equation. Agentic assistants do most of the typing, and every modern code host offers whitespace-ignoring diffs. What remains is the daily experience of reading code.

---

## 🪄 Install & Usage

```bash
uv tool install prose-formatter
```

```bash
prose format path/              # rewrite files in place
prose check path/               # exit non-zero on violations
prose format --diff path/       # show the diff without writing
prose check --stdin < file.py   # read from stdin
```

---

## 🪶 Rules

Eight rules ship in `0.1.0`:

| Rule | Coverage |
|---|---|
| `align-colons` | Collection literals, Pydantic / dataclass fields, function signatures, and docstring `Args:` sections |
| `align-equals` | Consecutive assignments at the same indentation |
| `align-imports` | The `import` keyword in `from ... import ...` groups and `as` in `import ... as ...` groups |
| `alphabetize` | Classes, methods (*grouped dunders → properties → privates → publics*), enum members, Pydantic fields (*required then optional*), function parameters, keyword arguments, and `from` imports |
| `match-case-align` | Single-expression case bodies |
| `one-per-line-collections` | `dict`, `list`, and `set` literals, even when they fit inline |
| `singleton-rule` | Skips colon padding when only one item exists in the aligned group |
| `strip-trailing-commas` | Multi-line collections and signatures |

### Example

Before:

```python
from sklearn.cluster import AgglomerativeClustering
from loguru import logger
from collections import Counter

config = {"threshold": 0.7, "metric": "euclidean", "linkage": "ward", "n_clusters": None}

class Posting(BaseModel, extra="forbid"):
    title: str
    company: str
    location: str | None = None
    date_posted: date | None

    def render(self, separator: str, include_location: bool, include_date: bool) -> str: ...

    def _slug(self):
        return self.company.lower().replace(" ", "-")

    def key(self):
        return f"{self._slug()}-{self.date_posted}"
```

After:

```python
from collections     import Counter
from loguru          import logger
from sklearn.cluster import AgglomerativeClustering

config = {
    "linkage"    : "ward",
    "metric"     : "euclidean",
    "n_clusters" : None,
    "threshold"  : 0.7
}

class Posting(BaseModel, extra="forbid"):
    company     : str
    date_posted : date | None
    title       : str

    location: str | None = None

    def _slug(self):
        return self.company.lower().replace(" ", "-")

    def key(self):
        return f"{self._slug()}-{self.date_posted}"

    def render(
        self,
        include_date     : bool,
        include_location : bool,
        separator        : str
    ) -> str:
        ...
```

---

## ⚖️ Configuration

`[tool.prose]` in your `pyproject.toml`:

```toml
[tool.prose]
line-length    = 88
target-version = "py310"

[tool.prose.rules]
align-colons             = true
align-equals             = true
align-imports            = true
alphabetize              = true
match-case-align         = true
one-per-line-collections = true
singleton-rule           = true
strip-trailing-commas    = true
```

Every rule is independently toggleable.

---

## 🗺️ Composition

*Prose* runs as the second pass in a two-stage pipeline. The first pass owns tokens (*line wrapping, quote normalization, indentation, blank-line discipline*) and the second pass owns layout (*alignment, alphabetization, the singleton rule, one-entry-per-line collections, trailing-comma stripping*). [Ruff](https://docs.astral.sh/ruff/) is the canonical first pass, in that `ruff format` matches the token-level scope and its lint config shares the `pyproject.toml` root with `[tool.prose]`.

```bash
ruff format && prose format
```

Running *Prose* first is incorrect. *Prose*'s alignment math depends on already-settled line breaks, and an upstream re-wrap will undo per-line layout decisions, forcing a third pass.

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

Black formats, Flake8 lints, and isort sorts, so each pairs with *Prose* at a different layer:

| Tool | Pairing |
|---|---|
| [Black](https://black.readthedocs.io/) | Run Black with `--skip-magic-trailing-comma`, then *Prose* second. Black collapses collections that `one-per-line-collections` re-expands and preserves trailing commas that `strip-trailing-commas` removes |
| [Flake8](https://flake8.pycqa.org/) | Add `extend-ignore = E203, E221, E272` to `.flake8` or `setup.cfg` (*and `C812` if [`flake8-commas`](https://github.com/PyCQA/flake8-commas) is installed*). Flake8 inherits the same `pycodestyle` codes Ruff inherits |
| [isort](https://pycqa.github.io/isort/) | Run isort first, *Prose* second, with no configuration adjustment. *Prose* alphabetizes within isort's groups and aligns the `import` keyword that isort leaves un-aligned |

---

## 🪡 Integrations

Wire *Prose* into anything that runs on save, on commit, or in CI.

### CI

```yaml
- run: uv tool install prose-formatter
- run: prose check .
```

### Format on Save

Any editor that supports run-on-save (*VSCode's `runOnSave`, Vim's `autocmd BufWritePost`, JetBrains File Watchers*) can shell out to `prose format <file>`.

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

Swap `entry: prose format` for `entry: prose check` for the check-only variant.

---

## 🗜️ Development

### One-Time Setup

*Prose* uses [mise](https://mise.jdx.dev) to manage every toolchain and CLI through a single `mise.toml`. Install mise, wire it into your shell, then `mise install` provisions the rest.

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

Suggested user settings (apply to any Rust project):

```json
"rust-analyzer.check.command": "clippy",
"rust-analyzer.imports.granularity.group": "module"
```

</details>
