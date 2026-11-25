import msgspec


class Posting(msgspec.Struct):
    title: str
    company: str
    date_posted: str
