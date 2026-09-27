from typing import Callable, Optional


def measure(renderable: object) -> object:
    measurer: Optional[
        Callable[["Console", "ConsoleOptions"], "Measurement"]
    ] = getattr(renderable, "__rich_measure__", None)
    return measurer
