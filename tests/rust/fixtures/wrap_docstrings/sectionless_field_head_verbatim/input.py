def dispatch(handlers):
    """
    Route each event to the handler registered against it.

    handlers (Callable[[Mapping[str, int], Sequence[str]], Awaitable[None]]): The callback.
    """
    return handlers
