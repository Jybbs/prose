"""
An assignment inside `if TYPE_CHECKING:` is type-system-only, so the
file's type-checker boundary already owns it and the rule leaves it
alone.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    DEFAULT_BACKEND = "memory"
