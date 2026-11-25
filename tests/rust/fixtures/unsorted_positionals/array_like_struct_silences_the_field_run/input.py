import msgspec


class Point(msgspec.Struct, array_like=True):
    y: float
    x: float


class Pair(msgspec.Struct):
    y: float
    x: float
