"""
A run referencing a non-module-local root through a `Subscript`
chain skips entirely. The root `sys` is not a run target and is
not a module-local binding written before the run, so safety
cannot be proved.
"""

zebra = sys.argv[0]
alpha = 1
beta = 2
