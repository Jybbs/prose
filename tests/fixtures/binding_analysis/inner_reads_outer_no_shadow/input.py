def outer():
    x = 1

    def inner():
        return x

    return inner
