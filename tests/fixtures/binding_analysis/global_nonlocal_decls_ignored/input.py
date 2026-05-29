def globalized():
    global x
    x = 1


def nested():
    y = 0
    def inner():
        nonlocal y
        y = 1
    inner()
