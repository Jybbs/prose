from typing import Optional, cast


def narrow(value: object) -> object:
    return cast(Optional[int])


def widen(value: object) -> object:
    return cast(Optional[int], value)
