"""
`with` blocks recurse the same way other compound bodies do.
The body's `from`-import run sorts within the context-managed
scope without leaking into surrounding statements.
"""

with open("foo") as f:
    from zeta import a
    from alpha import b
    from beta import c
