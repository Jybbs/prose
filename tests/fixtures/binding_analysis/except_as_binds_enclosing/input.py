"""
An `except ... as` clause introduces a binding in the enclosing scope,
classified as `ExceptHandler`, with reads inside the body counted.
"""


try:
    open("missing")
except FileNotFoundError as e:
    print(e)
    print(e)
