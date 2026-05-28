"""
Top-level free functions alphabetize within their tight slice at
module level. A non-function statement between two functions breaks
the run, so each contiguous function block sorts independently.
Class bodies route their function members through the four-group
method ordering, so the module-level pass does not touch methods.
"""

def render(items):
    return items


def collect(source):
    pass


def aggregate(records):
    pass
