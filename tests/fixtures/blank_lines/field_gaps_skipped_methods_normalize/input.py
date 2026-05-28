"""
The class-scope rule fires only on adjacent `(method, method)` pairs.
A field-to-field or field-to-method gap carries no canonical count,
so only the inter-method gap normalizes to 1 blank line.
"""


class Posting:
    field: int = 1
    name: str = "x"



    def m1(self):
        return 1



    def m2(self):
        return 2
