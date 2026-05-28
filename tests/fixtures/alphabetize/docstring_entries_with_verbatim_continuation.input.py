"""
A `Raises:` entry whose description carries an indented code block
keeps the verbatim block attached to its parent entry through the
reorder. The block sits beneath its entry's hanging column, so the
walker reads it as a continuation rather than a sibling entry.
"""


def load(path):
    """Read the file at `path`.

    Raises:
        ValueError: Raised when the content is malformed, for example::

            >>> load("not a real file")
            Traceback (most recent call last):
            ValueError: malformed content

        OSError: Raised when the file cannot be opened.
    """
    return path
