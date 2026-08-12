import msgspec


class Posting(msgspec.Struct, kw_only=True):
    title: str
    company: str
    date_posted: str
