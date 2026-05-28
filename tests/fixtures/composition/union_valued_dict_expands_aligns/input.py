"""
A long single-line dict whose values are `Optional` and `Union`
expressions from `typing`. The outer dict overflows the inline
budget and the values carry legacy union syntax.

Rules:
- collection_layout
- alphabetize
- align_colons
"""

from typing import Optional, Union

ALIASES = {"primary": Optional[str], "fallback": Union[int, str], "trailing_value": Optional[int], "extended_label": Union[str, None]}
