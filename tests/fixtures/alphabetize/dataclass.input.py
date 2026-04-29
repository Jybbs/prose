"""
A `@dataclass`-decorated class has annotated field declarations
alphabetized by AST shape, not by decorator name. Required fields
land before optional, each sub-group sorted by name. The structural
detection covers attrs, msgspec, and any hand-rolled annotated
container with the same shape.
"""

from dataclasses import dataclass


@dataclass
class Posting:
    title: str = "Untitled"
    company: str
    description: str | None = None
    date_posted: str
