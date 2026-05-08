"""
The yapf: disable / yapf: enable aliases produce the same
suppressed span as fmt: off / fmt: on, leaving the bracketed
assignments verbatim.
"""

x = 1
foo = 2
bar_baz = 3

# yapf: disable
short = 1
much_longer_name = 2
mid = 3
# yapf: enable

a = 1
bb = 2
ccc = 3
