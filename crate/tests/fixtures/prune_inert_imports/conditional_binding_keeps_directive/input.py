from __future__ import annotations

import sys
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from foo import Bar

if sys.version_info >= (3, 11):
    Alias = int


def use(bar: Bar, alias: Alias) -> None:
    pass
