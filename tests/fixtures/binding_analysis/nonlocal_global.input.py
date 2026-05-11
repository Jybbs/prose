"""
`global` and `nonlocal` declarations are ignored. Subsequent writes
to the declared names bind in the local function scope without
rewriting outward.
"""


def globalized():
    global x
    x = 1


def nested():
    y = 0
    def inner():
        nonlocal y
        y = 1
    inner()
