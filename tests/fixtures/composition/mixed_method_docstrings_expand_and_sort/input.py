"""
A class whose methods, declared out of alphabetical order, carry a
mix of single-line and multi-line docstrings. The full subset
reorders the methods, expands single-line docstrings into multi-line
form, places multi-line opener and closer on their own lines, and
settles the blank-line cushion between methods.

Rules:
- alphabetize
- no_single_line_docstrings
- multi_line_docstrings
- blank_lines
"""


class Service:
    def zeta(self):
        """Run the zeta operation."""
        return 1
    def alpha(self):
        """The alpha entry point.
        Returns the configured value."""
        return 2
    def beta(self):
        """Return the beta result."""
        return 3
