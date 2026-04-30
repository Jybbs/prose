"""
Pydantic fields whose default arrives via `Annotated[type,
Field(default=...)]` or `Annotated[type, Field(default_factory=...)]`
register as optional. The structural check walks the annotation for
a nested `Call` carrying a `default` or `default_factory` keyword,
covering Pydantic, msgspec, attrs, and similar libraries without
naming the field constructor.
"""

from typing import Annotated

from pydantic import BaseModel, Field


class Posting(BaseModel):
    title: Annotated[str, Field(default="Untitled")]
    company: Annotated[str, Field(min_length=1)]
    tags: Annotated[list[str], Field(default_factory=list)]
    description: Annotated[str, Field(min_length=50)]
