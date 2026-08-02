class Single(object):
    def clear(self):
        self.data.clear()


class Twice(Mapping, object, object):
    def keys(self):
        return self.data.keys()
