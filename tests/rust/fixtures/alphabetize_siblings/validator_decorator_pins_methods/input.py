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
