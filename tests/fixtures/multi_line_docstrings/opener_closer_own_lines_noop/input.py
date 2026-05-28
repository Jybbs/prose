"""
Already-canonical multi-line docstring with opener and closer each
on their own line passes through unchanged. The rule fires only
when one of the two boundaries shares a line with body content.
"""


def greet():
    """
    Summary line.

    More detail across additional body lines.
    """
    return 1
