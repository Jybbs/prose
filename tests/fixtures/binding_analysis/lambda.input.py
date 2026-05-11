"""
A lambda opens its own function scope. Parameters bind inside the
lambda scope and shadow same-named outer bindings.
"""


x = 1
f = lambda x: x
