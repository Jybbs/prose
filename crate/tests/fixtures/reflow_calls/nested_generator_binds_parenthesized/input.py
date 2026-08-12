def inner(beta):
    return beta


def outer(alpha):
    return alpha


outer(inner(row for row in rows if row.enabled))
