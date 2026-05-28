"""
A tuple literal inside a function call keeps its own trailing
comma because tuples are out of scope for this rule. The call's
trailing comma is still stripped.
"""

singletons = make_pair(
    (1,),
    (2,),
)
