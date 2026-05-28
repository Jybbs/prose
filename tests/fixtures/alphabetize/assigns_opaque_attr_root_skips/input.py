"""
A run referencing a non-module-local root through an `Attribute`
chain skips entirely. The root `os` is not a run target and is
not a module-local binding written before the run, so safety
cannot be proved.
"""

zebra = os.environ
alpha = 1
beta = 2
