from dataclasses import dataclass
from typing import ClassVar


@dataclass
class Posting:
    RETRIES: ClassVar[int] = 3
    MAX_AGE = 90
    title: str
    company: str
