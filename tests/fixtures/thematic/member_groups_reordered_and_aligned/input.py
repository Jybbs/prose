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
