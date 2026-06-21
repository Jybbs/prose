@overload
def f(x: int) -> None: ...


@overload
def f(x: str) -> str: ...


def f(x):
    return x
