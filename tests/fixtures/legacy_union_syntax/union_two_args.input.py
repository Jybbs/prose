"""
`Union[int, str]` flags as the legacy form, with the diagnostic
recommending `int | str` joined by the PEP 604 pipe.
"""

from typing import Union

x: Union[int, str] = 0
