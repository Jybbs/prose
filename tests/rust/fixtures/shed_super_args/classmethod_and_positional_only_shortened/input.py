class Record:
    @classmethod
    def build(cls):
        return cls()

    def merge(self, /, other):
        return other


class Entry(Record):
    @classmethod
    def build(cls):
        return super(Entry, cls).build()

    def merge(self, /, other):
        return super(Entry, self).merge(other)
