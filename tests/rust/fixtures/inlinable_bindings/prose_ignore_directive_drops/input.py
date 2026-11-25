def basic(arg):
    x = expensive(arg)  # prose: ignore[inlinable-bindings]
    return x + 1
