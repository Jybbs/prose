from dataclasses import dataclass


@dataclass
class Posting:
    title: str
    company: str
    description: str | None = None
    date_posted: str = ""
