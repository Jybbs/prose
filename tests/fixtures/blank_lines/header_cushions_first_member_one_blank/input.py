"""
A class header carries 1 blank line of separation before its first
non-docstring member. The cushion applies whether the first member is
a method, a decorated method, an annotated field, an unannotated
assignment, or a nested class. When the first member is a decorated
method, the cushion sits above the topmost decorator.
"""


class WithMethod:
    def alpha(self):
        return self


class WithDecoratedMethod:
    @classmethod
    def builder(cls):
        return cls()


class WithAnnotatedField:
    capacity: int = 0


class WithBareField:
    label = "unset"


class WithNestedClass:
    class Inner:
        def m(self):
            return self
