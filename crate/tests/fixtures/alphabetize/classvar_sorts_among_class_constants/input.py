from typing import ClassVar


class Config:
    TIMEOUT = 30
    RETRIES: ClassVar[int] = 3
    host: str
    port: int
