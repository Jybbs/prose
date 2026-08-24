class Command:
    def handler_map(self) -> dict[str, Callable[[Values, list[str]], None]]:
        return {}
