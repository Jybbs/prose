---
description: "The rules that reshape PEP 257 docstrings, their bodies and their quotes."
---

# Docs Rules

The docs rules rewrite the PEP 257 docstrings the [[docstring]] walker finds, reshaping the body, the quote placement, or both. Description prose between the opening `"""` and the first section heading wraps to `docstring-line-length` *(default 76)*, whereas every Title-case-headed section after it wraps to `code-line-length` *(default 88)*, and both take one budget when `docstring-structured-policy = "docstring-line-length"`.

<RuleCardList family="docs" />

The [**Configuration**](/reference/configuration#docstring-budgets) reference covers the two docstring budgets and the `docstring-structured-policy` key. The [[docstring]] primitive page covers the walker that finds every docstring in source order.
