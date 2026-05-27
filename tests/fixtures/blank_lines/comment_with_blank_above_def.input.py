"""
A comment sitting between two statements is treated as the second
statement's leading block. The rule normalizes the canonical gap
above the comment and binds the comment tight against the following
statement, collapsing any blank-count drift in the source.
"""


def first():
    return 1


# describes second




def second():
    return 2
