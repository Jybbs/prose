def outer():
    counter = 0

    def inner():
        nonlocal counter
        bumped = counter + 1
        return bumped

    return inner
