"""
A trailing end-of-line comment on the predecessor stays on its line
without joining the next def's leading region. The blank-line rewrite
lands between the two statements.
"""


def first():
    return 1  # short note

def second():
    return 2
