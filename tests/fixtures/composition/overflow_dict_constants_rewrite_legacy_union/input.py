"""
Module-level constants whose RHS dicts overflow the budget and whose
values carry legacy union syntax.

Rules:
- collection_layout
- alphabetize
- align_colons
- align_equals
- legacy_union_syntax
"""

from typing import Optional, Union

PRIMARY = {"alpha": Optional[str], "beta": Union[int, str], "gamma": Optional[int], "delta_long": Union[bool, None]}
SECONDARY = "fallback"
