"""
A bare `Optional[int]` flags as the legacy form. The diagnostic message
spells out the recommended `int | None` shape, leaving the source
untouched for the human to migrate.
"""

from typing import Optional

x: Optional[int] = None
