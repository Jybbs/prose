"""
A nested function reading an outer-scope name attributes one usage to
the outer binding, leaving the inner scope free of a shadowing entry.
"""


def outer():
    x = 1

    def inner():
        return x

    return inner
