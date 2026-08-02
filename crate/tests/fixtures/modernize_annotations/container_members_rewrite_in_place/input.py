import os

from collections import deque
from typing      import Callable, List, Optional, Tuple


def handle(fn: Callable[[Optional[int]], List[str]]) -> None: ...


def mix(value: Optional[List[int] | str]) -> None: ...


def pack(values: Tuple[List[int], *Ts]) -> None: ...
