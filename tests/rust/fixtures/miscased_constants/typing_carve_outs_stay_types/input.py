from typing import Annotated, Callable, Literal

from pydantic import Field

Flag = Literal[True]
Level = Literal[1, 2, 3]
Offset = Literal[-1]
Encoding = Literal[b"gzip"]
Positive = Annotated[int, Field(gt=0)]
Handler = Callable[[int, str], bool]
AnyHandler = Callable[..., int]
Empty = tuple[()]
first_level = LEVELS[1]
default_flag = OPTIONS[True]
raw_codec = CODECS[b"gzip"]
