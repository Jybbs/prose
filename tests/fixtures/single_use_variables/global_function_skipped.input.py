"""
A function whose body declares `global` is skipped entirely, since
the scope analysis would need to follow the module chain to give an
accurate single-use reading.
"""


def updater():
    global counter
    next_value = counter + 1
    return next_value
