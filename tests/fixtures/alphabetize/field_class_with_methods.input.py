"""
A field-bearing class without validator decorators sorts both its
fields (required-then-optional) and its methods (four-group). The
method-skip rule fires only when at least one method carries a
positional-binding decorator, the structural signal for validator-
shaped methods that depend on declaration order.
"""

from pydantic import BaseModel


class Posting(BaseModel):
    title: str
    name: str
    company: str

    def reset(self):
        pass

    def display(self):
        pass

    def __init__(self, **data):
        super().__init__(**data)
