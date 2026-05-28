"""
A class-body `__name` assignment lives in the class scope, invisible to
nested methods, with its sibling method definition recording its own
function-scope binding for `self`.
"""


class C:
    __secret = 1

    def get(self):
        return self
