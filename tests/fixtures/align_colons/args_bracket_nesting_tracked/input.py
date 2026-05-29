def index(records: list, schema: dict, options: dict) -> None:
    """
    Writes records to the index.

    Args:
        records (List[Tuple[int, str]]): rows to write.
        schema (Dict[str, int]): column type map.
        options (Dict[str, str]): backend-specific settings.
    """
    _write(records, schema, options)
