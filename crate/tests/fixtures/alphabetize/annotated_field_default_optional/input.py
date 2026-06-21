from typing import Annotated

from pydantic import BaseModel, Field


class Posting(BaseModel):
    title: Annotated[str, Field(default="Untitled")]
    company: Annotated[str, Field(min_length=1)]
    tags: Annotated[list[str], Field(default_factory=list)]
    description: Annotated[str, Field(min_length=50)]
