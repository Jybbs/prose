class Cache(object):
    def clear(self):
        self.entries.clear()


class Registry(object):
    def register(self, name):
        self.entries.append(name)
