"""
A multi-line `BoolOp` inside a list comprehension is out of scope.
The rule skips the comprehension and leaves the inner comparison
operators verbatim.
"""

filtered = [
    x
    for x in xs
    if (
        x.kind == "task"
        and x.weight == 1
    )
]
