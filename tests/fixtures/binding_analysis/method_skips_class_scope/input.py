"""
A class-body binding is invisible to a nested method's lookup
chain. The method's reference to the same name walks past the class
scope and resolves to the outer binding instead.
"""


x = "outer"


class C:
    x = "inner"

    def get(self):
        return x
