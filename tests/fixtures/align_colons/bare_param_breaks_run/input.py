def combine(
    user: User,
    source: str,
    legacy,
    target_path: str,
    destination: Filesystem,
) -> None:
    _impl(user, source, legacy, target_path, destination)
