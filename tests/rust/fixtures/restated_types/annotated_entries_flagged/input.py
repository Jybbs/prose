def dial(host: str, timeout: float) -> Session:
    """
    Open a session against a remote.

    Args:
        host (str): The remote to dial.
        timeout (float): Seconds before the dial is abandoned.
    """
