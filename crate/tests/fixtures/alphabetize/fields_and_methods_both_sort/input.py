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
