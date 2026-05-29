def schedule(when: float, action: str, retries: int) -> None:
    """
    Queues an action.

    Args:
        when (float): unix timestamp.
        action (str): callable name to dispatch.
        retries (int): number of retry attempts on failure.
    """
    _enqueue(when, action, retries)
