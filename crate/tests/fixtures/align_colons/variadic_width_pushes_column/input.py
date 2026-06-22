def record(
    event_name: str,
    *args: Any,
    timestamp: float,
    **kwargs: object,
) -> None:
    _store(event_name, *args, timestamp=timestamp, **kwargs)
