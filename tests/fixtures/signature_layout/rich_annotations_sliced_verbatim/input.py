from collections.abc import Callable, Mapping
from typing import Annotated, Any, Optional


def configure(callback: Callable[[int, str], bool], config: Mapping[str, Any], metadata: Annotated[dict[str, int], "extra"], tags: tuple[str, ...], timeout: Optional[float] = None) -> tuple[bool, str]:
    return (True, "ok")
