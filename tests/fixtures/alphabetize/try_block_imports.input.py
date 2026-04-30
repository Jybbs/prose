"""
Imports inside `try` and `except` arms alphabetize independently
per arm. The handler arm sorts on its own without leaking into the
`try` body's sort, mirroring how module-level blank-line-separated
import runs each sort within their own bucket.
"""

try:
    from zeta import a
    from alpha import b
    from beta import c
except ImportError:
    from omega import d
    from delta import e
