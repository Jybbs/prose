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
