def merge(target, source, fallback=None, strict=True, key=None):
    pass


class Catalog:
    def update(self, record, key=None, *, atomic=True, retries=3):
        pass

    @classmethod
    def from_dict(cls, mapping, strict=False):
        pass
