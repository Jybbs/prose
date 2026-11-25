__all__ = [
    "render",
    "Posting",
    "Catalog",
]


class Buffer:
    __slots__ = ["size", "data"]

    def write(self):
        pass

    def read(self):
        pass
