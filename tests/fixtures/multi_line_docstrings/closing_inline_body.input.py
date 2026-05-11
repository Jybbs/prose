"""
Closing triple-quote sharing a line with the last body line moves
to the next line at the docstring's indent, leaving the body
content verbatim.
"""


def greet():
    """
    Summary line on its own line.
    Trailing line touches the closer."""
    return 1
