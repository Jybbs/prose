def peel(items):
    head, *rest = items
    return wrap(head, rest)
