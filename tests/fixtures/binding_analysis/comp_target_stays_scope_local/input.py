"""
A comprehension target sits in its own scope and never bleeds into the
module table, even when its name matches an existing module binding.
"""


xs   = [1, 2, 3]
x    = "outer"
vals = [x for x in xs]
