"""
Google-style `Args:` entries that include `*args` and `**kwargs`.
The `*` prefix is a valid entry-line starter, so the colons of the
variadic entries align alongside the named ones.
"""


def emit(channel, *args, **kwargs):
    """
    Emits a structured event.

    Args:
        channel: target stream.
        *args: positional payload.
        **kwargs: keyword payload.
    """
    _emit(channel, *args, **kwargs)
