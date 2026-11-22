from dataclasses import dataclass
from typing import TYPE_CHECKING


@dataclass
class Posting:
    title: str
    if TYPE_CHECKING:
        url: str
        company: str
