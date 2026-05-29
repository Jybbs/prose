def outer():
    def inner():
        x = compute()
        return x + 1

    return inner
