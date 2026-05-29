from pydantic import BaseModel


class Solo(BaseModel):
    only_field : str


class Pair(BaseModel):
    first  : str
    second : int


class WithMethod(BaseModel):
    sole_field : str

    def serialize(self) -> dict:
        return self.model_dump()
