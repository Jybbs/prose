"""
A class body with annotated field declarations out of alphabetical
order, varying name widths, and default values of varying widths.

Rules:
- alphabetize
- align_colons
- align_equals
"""


class Endpoint:
    timeout: float = 30.0
    host: str = "localhost"
    retry_count: int = 3
    backend_label: str = "primary"
