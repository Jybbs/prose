def zeta():
    return 1


def mid():
    return zeta()


type Scored = Annotated[int, mid()]


def delta():
    pass


def beta():
    pass
