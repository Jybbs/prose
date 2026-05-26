# Docs Rules

The docs rules read against the PEP 257 docstring set discovered by the [[docstring]] walker and reshape the body, the quote placement, or both. Description prose between the opening `"""` and the first section heading reads against `docstring-line-length` *(default 76)*, while structured sections *(`Args:`, `Returns:`, `Raises:`)* read against `code-line-length` *(default 88)* or both collapse to one budget when `docstring-structured-policy = "docstring-line-length"`.

<RuleCardList family="docs" />

For the docstring-budget duality and the `docstring-structured-policy` knob, see the [**Configuration**](/reference/configuration#docstring-budgets) reference. For the walker that finds every docstring in source order, see the [[docstring]] primitive page.
