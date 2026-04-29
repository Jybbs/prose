"""
`global` and `nonlocal` name lists alphabetize. Order is
semantically meaningless because both declarations affect every
listed name uniformly within the enclosing scope.
"""

def update():
    global counter, items, registry
    counter = 0


def reset_outer():
    items = []
    def inner():
        nonlocal items, registry, counter
        items.clear()
