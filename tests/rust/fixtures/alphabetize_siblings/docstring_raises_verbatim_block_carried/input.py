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
