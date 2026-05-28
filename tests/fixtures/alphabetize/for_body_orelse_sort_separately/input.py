"""
The `body` and `orelse` arms of a `for` loop each sort their
own contents independently. The `else:` clause runs only when
the loop terminates without a `break`, but the alphabetize rule
treats both arms as ordinary statement bodies for sort purposes.
"""

for i in items:
    from zeta import a
    from alpha import b
else:
    from omega import c
    from delta import d
