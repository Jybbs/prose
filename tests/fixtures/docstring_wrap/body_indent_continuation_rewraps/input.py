"""
An entry whose continuation lines sit at the section-body indent rather
than at the hanging column re-wraps into the hanging shape, so adopting
this rule normalizes both never-wrapped and old-style-wrapped docstrings
on one pass.
"""


def configure():
    """
    Summary line.

    Args:
        foo: A descriptive parameter that is already split across two source
        lines at the section body indent rather than under the description.
    """
