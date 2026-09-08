from typing import IO, List, Optional, Sequence, cast

Rows = Optional["List[int]"]

head: "Sequence[int]" = []


def read(stream):
    return cast("IO[str]", stream)


__all__ = ["Rows", "head", "read"]
