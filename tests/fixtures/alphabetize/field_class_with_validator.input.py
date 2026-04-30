"""
A field-bearing class with a validator-shaped method skips method
alphabetization entirely. The validator's positional-binding
decorator (`@field_validator("title")`) is the AST-shape signal
that methods on this class may carry declaration-order semantics,
so all methods stay in source order while fields still sort.
"""

from pydantic import BaseModel, field_validator


class Posting(BaseModel):
    title: str
    name: str

    @field_validator("title")
    def normalize_title(cls, v):
        return v.strip()

    @field_validator("name")
    def normalize_name(cls, v):
        return v.lower()

    def display(self):
        return f"{self.name}: {self.title}"
