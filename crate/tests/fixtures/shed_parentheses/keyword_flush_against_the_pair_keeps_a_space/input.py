def f(x, y):
    if x:
        return('<not given>')
    elif(y):
        return(x)
    assert(x), "msg"
    yield(x)
    return (x)if y else(x)
