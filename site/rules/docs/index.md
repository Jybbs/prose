---
description: "The rules that reshape PEP 257 docstrings, their bodies and their quotes."
---

# Docs Rules

The docs rules rewrite the PEP 257 docstrings the [[docstring]] walker finds, reshaping the body, the quote placement, or both. The walker skips an f-string, a t-string, and a bytes literal, none of which Python takes as a docstring, as well as an implicitly concatenated string, which Python does take, so no docs rule rewrites any of them.

<RuleCardList family="docs" />

The [**Configuration**](/reference/configuration#docstring-budgets) reference covers the two docstring budgets [[wrap-docstrings]] wraps to and the `docstring-structured-policy` key.
