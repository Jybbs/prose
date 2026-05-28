"""
Parenthesized child expressions inside a multi-item list keep their
explicit parens through both expansion and collapse. Without
`parenthesized_range`, slicing each child's `range()` would drop
the parens and turn `(-a) ** 2` into `-a ** 2`, which Python parses
as `-(a ** 2)` and flips the sign of the result.
"""

precedence = [(-a) ** 2, (b + c) ** 3, (d - e) ** 4, (f * g) ** 5, (h / i) ** 6, (j + k) ** 7, (l - m) ** 8]
short_precedence = [
    (-a) ** 2,
    (b + c) ** 3
]
