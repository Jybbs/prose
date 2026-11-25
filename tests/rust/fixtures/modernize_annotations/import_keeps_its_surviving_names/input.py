from typing import Optional, cast


def widen(value: Optional[int]) -> object:
    return cast(object, value)
