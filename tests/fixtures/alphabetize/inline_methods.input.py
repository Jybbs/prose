"""
Methods whose body sits on the same line as `def` recurse through
the body rewriter without the inner statement triggering a
same-line block-range panic. The body's first statement starts
mid-line, so its line_start sits before the enclosing function's
range start, and `block_range` has to skip the leading-comment
scan rather than constructing a negative range.
"""

class C:
    def zeta(self): return 1
    def alpha(self): return 2
    def mu(self): pass
