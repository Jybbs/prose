def outer(alpha, beta, gamma, delta):
    return (alpha, beta, gamma, delta)


def inner(east, north, south, west):
    return (east, north, south, west)


result = outer(alpha=1, beta=inner(east=2, north=3, south=4, west=5), gamma=6, delta=7)
