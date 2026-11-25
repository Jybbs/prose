from typing import Callable

MAX = 1
Seconds = int
Handler = Callable[[Seconds], None]
TIMEOUT = MAX + 1
