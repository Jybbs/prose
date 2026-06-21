class Registry:
    def register(self, name):
        self.entries.append(name)

    @classmethod
    def empty(cls):
        return cls()


def forward(*args, **kwargs):
    return helper(*args, **kwargs)
