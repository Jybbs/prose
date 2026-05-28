"""
A Yields entry naming the yielded value wraps the description at the
docstring budget with a hanging indent at the description's start column,
matching every other entry-carrying section.
"""


def stream():
    """
    Summary line.

    Yields:
        chunk: The next decoded chunk of the underlying byte stream, sized to whatever the upstream reader chose to surface in a single read.
    """
