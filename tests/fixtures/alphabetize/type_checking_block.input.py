"""
`from`-import runs inside `if TYPE_CHECKING` blocks alphabetize the
same way they do at module level. The compound-statement recursion
threads the enclosing scope into each sub-body, so module-scope
reorders fire inside `if`, `for`, `while`, `with`, `try`, and
`match` arms without needing per-shape special-cases.
"""

from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from zeta import alpha
    from alpha import beta
    from beta import gamma
