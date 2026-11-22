def factory():
    cap = compute()

    def inner(limit=cap):
        return limit

    return inner
