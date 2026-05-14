"""
A class whose methods each carry a single-line triple-quoted
docstring on one source line.

Rules:
- no_single_line_docstrings
- multi_line_docstrings
"""


class Service:
    def primary(self):
        """The primary entry point."""
        return 1
    def secondary(self):
        """The secondary entry point."""
        return 2
