"""
An unmatched fmt: off suppresses through end of file. Assignments
below it stay verbatim while the assignments above pad to their
group width.
"""

x = 1
foo = 2
bar_baz = 3

# fmt: off
short = 1
much_longer_name = 2
mid = 3
