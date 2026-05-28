"""
`Union[int, str, None]` flattens its explicit `None` arg into the
recommendation `int | str | None`, leaving the source untouched.
"""

from typing import Union

x: Union[int, str, None] = None
