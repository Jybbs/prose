CAPITALS = {
    "USA"    : "Washington",
    "France" : "Paris",
    "Japan"  : "Tokyo",
    "Spain"  : "Madrid",
}


class User:
    id        : int
    name      : str
    email     : str
    is_active : bool


def register(
    request_id : str,
    user       : User,
    priority   : int,
) -> None:
    """
    Registers the user.

    Args:
        request_id : correlation id.
        user       : subject of registration.
        priority   : dispatch ordering.
    """
    pass
