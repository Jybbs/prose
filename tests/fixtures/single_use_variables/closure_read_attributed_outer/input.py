def factory():
    helper = compute()
    return lambda x: x * helper
