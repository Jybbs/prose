from dataclasses import dataclass


@dataclass(kw_only=True)
class Request:
    timeout: float
    method: str
    body: bytes | None = None
