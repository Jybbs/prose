from pydantic import BaseModel


class Posting(BaseModel):
    title: str = "Untitled"
    company: str
    description: str | None = None
    date_posted: str
    location: str | None = None
    url: str
