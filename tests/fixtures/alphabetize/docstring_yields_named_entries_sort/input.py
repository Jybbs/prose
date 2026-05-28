"""
The `Yields:` section reorders alphabetically when each entry
carries a `name: description` shape.
"""


def stream(source):
    """Yield each record from `source`.

    Yields:
        token: The next token in the stream.
        offset: The byte offset of the current record.
        record: The parsed record body.
    """
    return source
