"""
Method-level single-line docstring expands with the method-body
indent, two indent steps deeper than the module column.
"""


class Bag:
    def empty(self):
        """Return whether the bag is empty."""
        return not self.items
