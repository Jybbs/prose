def parse():
    """
    Summary line.

    Raises:
        ValueError: When the input bytes do not form a valid token sequence and the parser cannot recover a structured representation from the surrounding context.
        IOError: When the underlying stream is closed before the parser reaches a terminating token.
    """
