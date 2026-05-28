"""
A prose: off / prose: on block sits between two assignment runs that
the alignment rules would otherwise touch. The block survives
verbatim while the runs above and below it pad to their group
widths. The native prose namespace mirrors the fmt namespace's
span semantics exactly.
"""

x = 1
foo = 2
bar_baz = 3

# prose: off
short = 1
much_longer_name = 2
mid = 3
# prose: on

a = 1
bb = 2
ccc = 3
