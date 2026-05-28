"""
A pragma comment routed through `ruff_python_trivia::is_pragma_comment`
is invisible to the rule, even when its body starts with a digit or
matches another tool's directive shape.
"""

# noqa: F401
# type: ignore
# pyright: ignore[reportMissingImports]


def process(payload):
    return payload
