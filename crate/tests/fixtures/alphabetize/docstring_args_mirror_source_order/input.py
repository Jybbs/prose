def merge(target, source, retries=3):
    """Apply ``source`` onto ``target``.

    Args:
        retries: Attempts before giving up.
        target: Mapping receiving the update.
        source: Mapping providing new values.
    """
