"""
A `with ... as` clause binds the as-target in the enclosing scope.
Reads of the binding inside the body resolve back to that target.
"""


def read_lines(path):
    with open(path) as f:
        return f.readlines()
