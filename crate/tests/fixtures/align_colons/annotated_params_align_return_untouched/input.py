def dispatch(
    request_id: str,
    user: User,
    priority: int,
    timeout: float,
) -> Response:
    return _impl(request_id, user, priority, timeout)
