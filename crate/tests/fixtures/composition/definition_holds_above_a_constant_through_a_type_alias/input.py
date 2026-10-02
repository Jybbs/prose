class Base:
    y = 1


type T = Base


class Zeta(T):
    pass


x = Base.y


class Zed:
    pass


class Beta:
    pass
