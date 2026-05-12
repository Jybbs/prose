"""
`typing.Optional[int]` after `import typing` resolves through the
attribute chain back to `typing.Optional` and flags the same as the
bare `Optional[int]` form.
"""

import typing

x: typing.Optional[int] = None
