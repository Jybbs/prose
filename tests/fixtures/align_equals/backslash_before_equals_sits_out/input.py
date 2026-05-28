"""
A backslash continuation between the target and the `=` sits out the
group, leaving neighboring assignments unaligned.
"""

foo \
    = 1
bar = 2
