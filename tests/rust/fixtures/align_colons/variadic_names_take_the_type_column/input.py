def emit(channel, *args, **kwargs):
    """
    Args:
        channel (str): target stream.
        *args (int): positional payload.
        **kwargs (Any): keyword payload.
    """
    _emit(channel, *args, **kwargs)
