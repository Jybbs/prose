def lookup(key) -> int | None:
    return registry.get(key)


def make_handler() -> Callable[..., None]:
    return default_handler
