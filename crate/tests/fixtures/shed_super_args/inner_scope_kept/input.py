class Base:
    def items(self):
        return []


class Child(Base):
    def deferred(self):
        return (lambda: super(Child, self).items())()

    def gathered(self):
        return [super(Child, self).items() for _ in range(1)]

    def wrapped(self):
        def inner():
            return super(Child, self).items()

        return inner()
