from numpy.typing import NDArray


class Box(Generic[T]):
    pass


path_sep = platform.path_sep
Vec = NDArray[float]
Crate = Box[int]
opener = TarFile.open

if path_sep:
    pass


def read(target: Vec, crate: Crate) -> None:
    opener(target, crate)
