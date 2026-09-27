from typing import Annotated


def zeta():
    return 1


def mid():
    return zeta()


type Scored = Annotated[int, mid()]


def delta():
    pass


print(Scored.__value__)


def beta():
    pass
