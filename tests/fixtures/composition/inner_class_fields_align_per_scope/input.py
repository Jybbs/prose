"""
A class containing a nested helper class, where each class scope
carries annotated fields out of order. alphabetize and the alignment
rules act independently within each scope.

Rules:
- alphabetize
- align_colons
- align_equals
"""


class Order:
    total: float = 0.0
    customer_id: str = ""
    placed_on: str = ""

    class LineItem:
        sku: str = ""
        quantity: int = 1
        price: float = 0.0
