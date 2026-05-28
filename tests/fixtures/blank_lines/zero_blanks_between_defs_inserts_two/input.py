"""
Zero blank lines between two module-level defs expand to the
canonical 2. The rule inserts the missing newlines into the
inter-statement whitespace.
"""


def first():
    return 1
def second():
    return 2
