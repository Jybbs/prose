import os
import tarfile

from numpy.typing import NDArray
from typing import Generic, TypeVar

T = TypeVar("T")


class Box(Generic[T]):
    pass


path_sep = os.sep
opener = tarfile.TarFile.open
Vec = NDArray[float]
Crate = Box[int]

if path_sep:
    pass


def read(target: Vec, crate: Crate) -> None:
    opener(target, crate)
