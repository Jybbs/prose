import os

from collections import deque
from typing      import Callable, List, Optional, Tuple

handler : Callable[[Optional[int]], List[str]] = fn
packed  : Tuple[List[int], *Ts] = ()
mixed   : Optional[List[int] | str] = None
