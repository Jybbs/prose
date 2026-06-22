def lookup():
    """
    Summary line.

    Returns:
        record: The matching row from the database when the lookup succeeds, populated from every selected column and ready for the caller to consume.
        cached: Whether the record came from the in-memory cache rather than a fresh query.
    """
