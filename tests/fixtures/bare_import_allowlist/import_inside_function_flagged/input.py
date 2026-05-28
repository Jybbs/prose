"""
The rule recurses into function bodies through `walk_stmt`, flagging
a non-allowlisted bare import nested inside `def` the same as a
module-level one.
"""


def show_version():
    import sys

    return sys.version
