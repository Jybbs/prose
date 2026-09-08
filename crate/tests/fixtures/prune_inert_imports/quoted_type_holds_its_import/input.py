from typing import IO, List, cast

x: "List[int]" = []


def read(stream):
    return cast("IO[str]", stream)


__all__ = ["read", "x"]
