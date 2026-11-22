from pydantic import BaseModel


class Zed:
    pass


class Beta(BaseModel):
    reference: Zed


class Alpha:
    def run(self, reference: Zed) -> Zed: ...
