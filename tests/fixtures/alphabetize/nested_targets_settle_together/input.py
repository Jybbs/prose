"""
Several alphabetize targets fire in one source: classes at module
level, methods within a class, kwargs at a call site, and a
`from`-import run. The rule iterates internally to settle nested
concerns inside an enclosing reorder.
"""

from loguru import logger
from collections import Counter


class Gamma:
    def public(self):
        return Counter(
            second=2,
            first=1,
        )


class Alpha:
    def __init__(self):
        pass

    def public(self):
        pass

    def __repr__(self):
        pass


class Beta:
    pass
