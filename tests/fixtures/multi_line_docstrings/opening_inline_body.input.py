"""
Body content that starts on the opening line moves to the next line
at the docstring's indent, leaving the opening triple-quote alone on
its own line.
"""


def greet():
    """Summary line starts inline with the opener.
    Trailing line sits at the docstring's indent.
    """
    return 1
