"""
A parenthesized argument expression has its own closing `)` between
the inner expression and the trailing comma. The backward scan
starts from the call's outer `)` and reads the trailing comma
directly, so the inner `)` does not need to be skipped explicitly.
"""

result = compute(
    (a + b),
    (c * d),
)
