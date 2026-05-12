"""
A non-heading line at the docstring body indent within a section ends
the section and resumes description prose at the description budget,
even when no blank line separates the section body from the prose.
"""


def example():
    """
    Summary.

    Args:
        foo: short description.
    Continued description prose at the body indent that exceeds the seventy six character description budget wraps at that budget.
    """
    return 1
