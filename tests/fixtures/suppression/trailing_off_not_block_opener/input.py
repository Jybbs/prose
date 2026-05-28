"""
A trailing fmt: off comment is a normal end-of-line comment, not
a block-opener. The line carrying it still receives its
alignment edit, and code after it is not suppressed.
"""

x = 1
foo = 2
short = 3  # fmt: off
aa = 4
bbb = 5
cccc = 6
