def merge(primary, fallback):
    return primary


class Loader:
    def ready(self):
        return True


class CachedLoader(Loader):
    def ready(self):
        if super(CachedLoader, self).ready:
            value = merge(primary,
                          fallback)
            return value
        return False
