class Order:
    total: float = 0.0
    customer_id: str = ""
    placed_on: str = ""

    class LineItem:
        sku: str = ""
        quantity: int = 1
        price: float = 0.0
