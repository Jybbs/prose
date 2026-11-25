from dataclasses import KW_ONLY, dataclass
from typing import ClassVar


@dataclass
class Session:
    REGISTRY: ClassVar[dict] = {}
    alpha: int
    beta: int
    _: KW_ONLY
    zulu: str
    yankee: str
