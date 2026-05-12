"""
`from typing import Optional as Opt` binds `Opt` to `typing.Optional`,
so `Opt[int]` resolves and flags through the alias.
"""

from typing import Optional as Opt

x: Opt[int] = None
