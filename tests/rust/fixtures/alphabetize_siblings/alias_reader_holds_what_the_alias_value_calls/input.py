from typing import Annotated

type Scored = Annotated[int, mid()]


def zeta():
    return 1


def mid():
    return zeta()


def delta():
    pass


print(Scored.__value__)


def beta():
    pass
