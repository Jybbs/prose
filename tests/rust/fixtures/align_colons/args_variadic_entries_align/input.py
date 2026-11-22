def emit(channel, *args, **kwargs):
    """
    Emits a structured event.

    Args:
        channel: target stream.
        *args: positional payload.
        **kwargs: keyword payload.
    """
    _emit(channel, *args, **kwargs)
