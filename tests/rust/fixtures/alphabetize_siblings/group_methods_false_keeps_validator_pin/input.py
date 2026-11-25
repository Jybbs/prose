from pydantic import BaseModel, field_validator


class Posting(BaseModel):
    title: str
    name: str

    def display(self):
        return f"{self.name}: {self.title}"

    @field_validator("name")
    def check_name(cls, v):
        return v.lower()
