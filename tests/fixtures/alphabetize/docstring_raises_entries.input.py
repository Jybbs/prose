"""
The `Raises:` section reorders alphabetically by exception name.
"""


def load(path):
    """Load configuration.

    Raises:
        ValueError: When the file is empty.
        OSError: When the path cannot be opened.
        KeyError: When a required field is missing.
    """
    return path
