from typing import Callable, Optional


def measure(renderable: object) -> object:
    get_console_width: Optional[
        Callable[["Console", "ConsoleOptions"], "Measurement"]
    ] = getattr(renderable, "__rich_measure__", None)
    return get_console_width
