"""
A name rebound across the arms of an `if` collapses into a single
module-scope binding with two write offsets, regardless of which arm
runs at runtime.
"""


cond = True
if cond:
    x = 1
else:
    x = 2
print(x)
