"""
A relative `from . import annotations` carries `level > 0`, so the
detector skips it. The directive-shaped import resolves against the
package, not `__future__`, and stays in place.
"""

from . import annotations

x = annotations
