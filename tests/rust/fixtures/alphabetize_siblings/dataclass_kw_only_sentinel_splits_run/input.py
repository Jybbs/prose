from dataclasses import KW_ONLY, dataclass


@dataclass
class Posting:
    title: str
    company: str
    _: KW_ONLY
    url: str
    date_posted: str
