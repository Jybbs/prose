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
