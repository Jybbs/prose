"""
A walrus target inside a comprehension lifts to the nearest
non-comprehension scope. The comprehension target stays scope-local
while the walrus target becomes a module-level binding.
"""


xs = [1, 2, 3]
vals = [y for x in xs if (y := x * 2) > 0]
