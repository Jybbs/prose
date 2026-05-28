"""
Outer class with an inner helper class, where each class carries
field declarations and methods in arbitrary order. The full pipeline
sorts fields and methods at every nesting depth, aligns the
declaration columns within each scope independently, and settles
the blank-line cushion between methods.
"""


class Order:
    total: float = 0.0
    customer_id: str = ""
    placed_on: str = ""

    class LineItem:
        sku: str = ""
        price: float = 0.0
        quantity: int = 1
        def subtotal(self):
            return self.price * self.quantity
    def total_with_tax(self, rate):
        return self.total * (1 + rate)
    def __repr__(self):
        return f"Order({self.customer_id})"
