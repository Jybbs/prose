"""
Function signature with annotated `*args` and `**kwargs` alongside
named annotated parameters. The `*` and `**` prefixes count toward
each variadic parameter's display width, so `**kwargs` pushes the
target column out and the named parameters pad to match.
"""

def record(
    event_name: str,
    *args: Any,
    timestamp: float,
    **kwargs: object,
) -> None:
    _store(event_name, *args, timestamp=timestamp, **kwargs)
