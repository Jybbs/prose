def outer():
    def inner_a():
        return 1
    def inner_b():
        return 2
    return inner_a, inner_b
