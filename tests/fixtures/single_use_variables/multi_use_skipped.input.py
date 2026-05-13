"""
A function-local binding read more than once is not flagged, since
the reader benefits from the named handle at every use site.
"""


def doubled():
    x = compute()
    return x + x
