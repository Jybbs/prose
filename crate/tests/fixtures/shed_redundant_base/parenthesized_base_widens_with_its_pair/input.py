class Adapter(Mapping, (object)):
    def get(self, key):
        return self.data[key]


class Wrapper((object), Sized):
    def size(self):
        return len(self.data)
