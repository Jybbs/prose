def flags(commands: list[tuple[str, ...]]) -> dict[str, bool]:
    return {
        "pr": any(
            args[:3] in {("gh", "pr", "create"), ("gh", "pr", "edit"), ("gh", "pr", "view")} for args in commands
        )
    }
