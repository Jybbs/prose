"""
`from`-import and bare `import` runs alphabetize inside function
bodies just like at module scope. Function bodies otherwise skip
class, method, and field reorders because their statements carry
sequential semantics, but Python imports are idempotent and order-
independent, so import-run sorts apply uniformly across every
body shape.
"""

def f():
    from zeta import a
    from alpha import b
    return a + b


def g():
    import zlib
    import argparse
    return argparse, zlib
