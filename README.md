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
from collections import Counter
from sklearn.cluster import AgglomerativeClustering
from loguru import logger

config = {"linkage": "ward", "metric": "euclidean", "n_clusters": None, "threshold": 0.7}

class Posting(BaseModel, extra="forbid"):
    title: str
    company: str
    location: str | None = None
    date_posted: date | None
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

*Prose* works well as a second pass after any other Python formatter that owns line length and quote normalization. If you already use one, configure it to skip trailing-comma enforcement and let *Prose* handle alignment, ordering, and the singleton rule on top. See `docs/interop.md` for specifics.

---

## 🗜️ Development

Requires Rust 1.80+ and Python 3.10+.

```bash
cargo build
cargo test
cargo insta review
maturin develop
```

