"""
A class body whose statements live inside `if` / `else` arms
sorts each arm under class scope. Methods inside an arm sort
into the four-group ordering, and annotated field declarations
sort required-then-optional, the same way they would at the
class's top level.
"""

class C:
    if FLAG:
        def beta(self): pass
        def __init__(self): pass
        def alpha(self): pass
    else:
        x: int = 1
        z: int
        y: int = 2
