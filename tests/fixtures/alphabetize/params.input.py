"""
Function parameters alphabetize within two sub-groups: required (no
default) before optional (has default). `self` and `cls` pin first
in method parameter lists. The fixture exercises a free function and
two methods so all three pin shapes show up in one snapshot.
"""

def merge(target, source, fallback=None, strict=True, key=None):
    pass


class Catalog:
    def update(self, record, key=None, *, atomic=True, retries=3):
        pass

    @classmethod
    def from_dict(cls, mapping, strict=False):
        pass
