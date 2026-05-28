"""
A chain of own-line comments between two statements forms a single
leading comment block for the second statement. The rule normalizes
2 blank lines above the topmost comment and binds the bottommost
comment tight against the following def.
"""


def first():
    return 1
# explains why
# bookkeeping
# second helper
def second():
    return 2
