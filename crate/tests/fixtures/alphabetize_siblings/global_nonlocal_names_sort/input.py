def update():
    global counter, items, registry
    counter = 0


def reset_outer():
    counter, items, registry = 0, [], {}
    def inner():
        nonlocal items, registry, counter
        items.clear()
