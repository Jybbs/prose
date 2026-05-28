"""
`Args:` entries whose parenthesized types contain bracketed
generics. The colon detector tracks both `()` and `[]` nesting, so
colons inside `Dict[str, int]` and `List[Tuple[int, str]]` do not
fool the parser. Alignment anchors on the entry's outer colon.
"""

def index(records: list, schema: dict, options: dict) -> None:
    """
    Writes records to the index.

    Args:
        records (List[Tuple[int, str]]): rows to write.
        schema (Dict[str, int]): column type map.
        options (Dict[str, str]): backend-specific settings.
    """
    _write(records, schema, options)
