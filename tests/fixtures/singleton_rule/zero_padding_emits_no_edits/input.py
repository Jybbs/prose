"""
Input where every singleton context already carries zero pre-`:`
padding. The rule recognizes each gap as already empty and emits no
edits, so the output text equals the input text byte-for-byte and
the pipeline reports no change.
"""

def fetch(timeout: float) -> bytes:
    return b""


single = {"only_key": compute_value()}


class Solo:
    only_field: str


def render(template):
    """Expands the template with substitutions.

    Args:
        template: The template string.
    """
