"""
A class with a single annotated field plus a validator-shaped
method does not pin its methods. The structural pin requires at
least two annotated fields, so single-field containers (which are
rarely true data classes) keep their method alphabetization. This
pins the lower bound of the field-bearing detection.
"""

from pydantic import BaseModel, field_validator


class Counter(BaseModel):
    value: int

    def reset(self):
        self.value = 0

    @field_validator("value")
    def normalize(cls, v):
        return max(v, 0)

    def __init__(self, **data):
        super().__init__(**data)
