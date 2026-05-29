from typing import TypedDict


class Config(TypedDict):
    timeout: int
    name: str
    retries: int
    host: str
