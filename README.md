<div align="center">
<img src="assets/title.svg" alt="Prose" width="800">
<h3><em>A Python typesetter for the reader</em></h3>

[![Rust 1.80+](https://img.shields.io/badge/rust-1.80+-8a80cb.svg)](https://www.rust-lang.org/)
[![Python 3.10+](https://img.shields.io/badge/python-3.10+-8a80cb.svg)](https://www.python.org/)
[![maturin](https://img.shields.io/badge/built%20with-maturin-8a80cb.svg)](https://www.maturin.rs/)
[![License: MIT](https://img.shields.io/badge/license-MIT-8a80cb.svg)](./LICENSE)

</div>

---

## 🪻 About

*Prose* formats Python source to be *legible at a glance*. It aligns equals signs and colons vertically across consecutive lines, places one entry per line in dictionaries and lists, alphabetizes methods and fields within their groups, applies a singleton rule for colon padding, and treats code like prose rather than minified text.

**Status:** pre-alpha. Under active development. No public release yet.

---

## 🗞️ Philosophy

Code is read far more often than it is written. A reader's eye moves down a page and across adjacent lines looking for parallels, patterns, and shape. When every `=` sits at a different column and every collection is compressed onto one line, that shape disappears, leaving the reader to reconstruct it character by character. *Prose* restores the shape. Aligned columns let the eye skim. One-per-line collections make each entry a unit. Alphabetized groupings give every reader the same predictable landmarks.

The trade-offs the minimalist formatters were built to avoid (*wider diffs, more vertical scrolling, occasional re-alignment churn*) no longer dominate the equation. Agentic assistants do most of the typing now, and every modern code host offers whitespace-ignoring diffs. What remains is the daily experience of reading code, and *Prose* is built for that experience.

---

## 🪄 Install & Usage

```bash
uv tool install prose
```

Not yet published.

```bash
prose format path/
prose check path/
prose format --diff path/
prose check --stdin < file.py
```

---

## 🪶 Rules

The `0.1.0` release ships eight rules:

1. **Align `:`** in collection literals, Pydantic / dataclass fields, function signatures, and docstring Args sections
2. **Align `=`** across consecutive assignments at the same indentation
3. **Align `import`** keyword in `from ... import ...` groups and `as` in `import ... as ...` groups
4. **Alphabetize** classes in a module, methods within a class (*grouped dunders → properties → privates → publics*), enum members, Pydantic fields (*required then optional*), function parameters, keyword arguments, and `from` imports
5. **Match-case alignment** when every case body is a single expression
6. **One entry per line** for dict / list / set literals, even when they fit inline
7. **Singleton rule** skips colon padding when only one item exists in the aligned group
8. **Strip trailing commas** in multi-line collections and signatures

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

## 🗺️ Using *Prose* with another formatter

*Prose* works well as a second pass after any other Python formatter that owns line length and quote normalization. If you already use one, configure it to skip trailing-comma enforcement and let *Prose* handle alignment, ordering, and the singleton rule on top.

---

## 🗜️ Development

### One-time setup

*Prose* uses [mise](https://mise.jdx.dev) to manage every toolchain and CLI the project depends on through a single `mise.toml`. After installing mise once and wiring it into your shell, `mise install` provisions everything else.

**1. Install mise:**

```bash
curl https://mise.run | sh
```

The installer drops the binary into `~/.local/bin/mise`.

**2. Wire mise into zsh.** Three init files cover three load contexts (*every shell, login shells, interactive shells*) so mise-managed tools resolve correctly whether you are inside an interactive terminal, a login shell, or a non-interactive subprocess:

```bash
# ~/.zshenv  (sourced for every shell, including non-interactive)
export PATH="$HOME/.local/bin:$PATH"

# ~/.zprofile  (sourced for login shells, before .zshrc)
eval "$(mise activate zsh --shims)"

# ~/.zshrc  (sourced for interactive shells)
eval "$(mise activate zsh)"
```

`.zshenv` puts `mise` itself on `PATH` so the later `eval` lines can find it. `.zprofile`'s `--shims` activation makes mise-managed binaries resolvable in non-interactive contexts (*scripts, editors, GUI launches*). `.zshrc`'s full activation gives interactive shells the per-directory tool resolution and task discovery.

**3. Clone and provision:**

```bash
git clone https://github.com/Jybbs/prose.git
cd prose
mise install
```

`mise install` provisions:

| Tool | Purpose |
|---|---|
| `rust` (stable) | Rust toolchain via rustup |
| `python` (3.14) | Python interpreter for wheel builds |
| `uv` | Python package and venv manager |
| `maturin` | Rust → Python wheel builder |
| `cargo-insta` | Snapshot test review |

### Daily workflow

Tasks are defined in `mise.toml` and discoverable via `mise tasks`:

| Command | What it does |
|---|---|
| `mise build` | Compile in debug mode |
| `mise test` | Run all tests including `insta` snapshots |
| `mise review` | Interactively accept pending snapshot diffs |
| `mise wheel` | Build the wheel and install into `.venv` |
| `mise lint` | Run `clippy` with all warnings as errors |
| `mise format` | Format Rust source with `rustfmt` |
| `mise check` | Verify Rust source matches `rustfmt` without rewriting |
| `mise ci` | Lint + test + wheel (full local sweep) |

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

