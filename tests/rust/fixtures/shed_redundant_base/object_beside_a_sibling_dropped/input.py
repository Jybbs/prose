class Layered(Mapping, Sized, object):
    def values(self):
        return self.data.values()


class Trailing(Mapping, object):
    def keys(self):
        return self.data.keys()
