"""
A class body interleaves methods with class-level constants. The
constants pin in their source slots and the methods sort around
them, redistributing into the slots vacated by other methods.
"""

class Settings:
    DEFAULT_TIMEOUT = 30

    def update(self, key, value):
        pass

    MAX_RETRIES = 5

    def _read_lock(self):
        pass

    def __init__(self):
        pass
