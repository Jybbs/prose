"""
Chained assignment, tuple unpacking, and attribute targets all
fall outside the `[Expr::Name(target)]` single-target shape the
rule flags. Each line passes through silently.
"""

A = B = 1
A, B = 1, 2
FOO.bar = 1
