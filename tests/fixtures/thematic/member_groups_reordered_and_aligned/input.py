"""
Service class with annotated fields out of alphabetical order, a
property, a private helper, and two public methods declared in
arbitrary order. The full pipeline reorders methods into the
canonical dunder / property / private / public groups, alphabetizes
the annotated fields, aligns their `:` and `=` columns, and
normalizes the blank-line spacing between members.
"""


class CacheService:
    capacity: int = 128
    timeout: float = 30.0
    backend_name: str = "memory"
    def evict(self, key):
        return self._store.pop(key, None)
    def get(self, key):
        return self._store.get(key)
    @property
    def size(self):
        return len(self._store)
    def _initialize_store(self):
        self._store = {}
