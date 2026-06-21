def update():
    global counter, items, registry
    counter = 0


def reset_outer():
    items = []
    def inner():
        nonlocal items, registry, counter
        items.clear()
