from dataclasses import dataclass
from typing import ClassVar


@dataclass
class Palette:
    zebra: int
    DEFAULT: ClassVar[str] = "muted"
    apple: int
