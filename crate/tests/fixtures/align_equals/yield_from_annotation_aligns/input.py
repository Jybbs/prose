def stream(items):
    forwarded: (yield from items) = 1
    received_value_field: int = 2
