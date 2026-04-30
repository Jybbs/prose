"""
Pydantic fields split into required (no default) and optional (has
default) sub-groups. Each sub-group alphabetizes independently and
the required group lands above the optional group regardless of
source order.
"""

from pydantic import BaseModel


class Posting(BaseModel):
    title: str = "Untitled"
    company: str
    description: str | None = None
    date_posted: str
    location: str | None = None
    url: str
