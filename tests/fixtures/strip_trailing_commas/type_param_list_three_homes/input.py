def transform[
    T,
    U,
](x: T) -> U:
    raise NotImplementedError


class Container[
    T,
    U,
]:
    pass


type Pair[
    T,
    U,
] = tuple[T, U]
