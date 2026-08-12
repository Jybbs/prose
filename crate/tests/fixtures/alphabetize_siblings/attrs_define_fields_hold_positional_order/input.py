import attrs


@attrs.define
class Posting:
    title: str
    company: str
    date_posted: str
