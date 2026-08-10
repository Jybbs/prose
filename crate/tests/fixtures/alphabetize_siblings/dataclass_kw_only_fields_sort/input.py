from dataclasses import dataclass


@dataclass(kw_only=True)
class Posting:
    title: str
    company: str
    date_posted: str
