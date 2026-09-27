from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from typing import IO

trace_file: IO[str] | None = None
