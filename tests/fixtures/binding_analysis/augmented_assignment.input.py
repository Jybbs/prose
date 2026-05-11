"""
An augmented assignment counts as one read plus one write, so the
target's `assignment_count` jumps and its kind list carries `AugAssign`
alongside the initial `Assignment`.
"""


x  = 0
x += 1
x += 2
