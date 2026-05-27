<div align="center">
<img src="site/public/title-with-tagline.svg" alt="Prose, a Python typesetter for the reader." width="800">

[![Rust](site/public/badges/rust.svg)![1.82+](https://img.shields.io/badge/1.82+-8a80cb?style=for-the-badge)](https://www.rust-lang.org/)
[![Python](site/public/badges/python.svg)![3.10+](https://img.shields.io/badge/3.10+-8a80cb?style=for-the-badge)](https://www.python.org/)
[![Coverage](site/public/badges/coverage.svg)![percent](https://img.shields.io/codecov/c/github/Jybbs/prose?style=for-the-badge&label=&color=8a80cb)](https://codecov.io/gh/Jybbs/prose)
[![Documentation](site/public/badges/docs.svg)![Docs](https://img.shields.io/badge/Docs-8a80cb?style=for-the-badge)](https://prose.fyi/)

</div>

---

## 🪻 About

***Prose*** formats Python source to be *legible at a glance*. It aligns equals signs and colons vertically across consecutive lines, places one entry per line in dictionaries and lists, alphabetizes methods and fields within their groups, applies a singleton rule for colon padding, and treats code like prose rather than minified text.

> [!NOTE]
> ***Prose*** is still pre-1.0. The rule catalog and configuration knobs continue to grow across release lines.

---

## 🗞️ Philosophy

Code is read far more often than it is written. A reader's eye moves down a page and across adjacent lines looking for parallels, patterns, and **shape**. When every `=` sits at a different column and every collection is compressed onto one line, that shape disappears, forcing the eye to slow down. ***Prose*** restores it, with aligned columns letting the eye skim, one-per-line collections making each entry a unit, and alphabetized groupings giving every reader the same landmarks.

The trade-offs minimalist formatters were built to avoid (*wider diffs, more vertical scrolling, occasional re-alignment churn*) no longer dominate the equation. Agentic assistants do much of the typing, and every modern code host offers whitespace-ignoring diffs. What remains is the daily experience of reading code.

---

## 🪄 Install & Usage

```bash
uv tool install prose-formatter
```

The binary exposes `format`, `check`, `cache`, and `completions`:

```bash
prose format path/             # rewrite files in place
prose check path/              # exit non-zero on violations
prose format --diff path/      # show the diff without writing
prose check --stdin < file.py  # read from stdin
prose format - < file.py       # `-` reads from stdin too
```

---

## 📰 Further Reading

The full edition lives at [prose.fyi](https://prose.fyi/):

- The [**rule catalog**](https://prose.fyi/rules/) walks every rule with before/after fixtures and per-knob configuration.
- The [**configuration reference**](https://prose.fyi/reference/configuration) lists every `[tool.prose]` key and per-rule sub-table.
- The [**cache reference**](https://prose.fyi/reference/cache) covers the cache directory, `--no-cache`, the `[tool.prose.cache]` table, and the `prose cache` subcommands.
- The [**exit-code matrix**](https://prose.fyi/reference/exit-codes) is the contract CI gates and pre-commit hooks compile against.
- [**Suppression directives**](https://prose.fyi/usage/suppression) cover `# prose: off`, `# prose: skip`, and the rest of the directive surface.
- [**Composition with Ruff**](https://prose.fyi/integrations/ruff) pairs the token-level formatter with `prose format`.
- [**Editor, pre-commit, and CI integrations**](https://prose.fyi/integrations/) wire *Prose* into the development loop.

---

## 🗜️ Development

*Prose* is a Rust crate that ships as a Python wheel through [**maturin**](https://www.maturin.rs/), with [**mise**](https://mise.jdx.dev) managing the Rust toolchain, Python interpreter, and every supporting CLI through a single `mise.toml`. After installing mise and [**activating it in your shell**](https://mise.jdx.dev/installing-mise.html), three commands provision the rest:

```bash
git clone https://github.com/Jybbs/prose.git
cd prose
mise install
```

`mise tasks` lists every available task, and `mise ci` runs the full local sweep that mirrors GitHub Actions.

For the architecture, the [**primitive surface**](https://prose.fyi/primitives/) walks every public type (*`Source`, `Pipeline`, `BindingAnalysis`, `SuppressionMap`, `RuleId`, `Edit`*), and the [**pipeline order**](https://prose.fyi/reference/pipeline-order) explains how each rule reads a settled AST between reparses.
