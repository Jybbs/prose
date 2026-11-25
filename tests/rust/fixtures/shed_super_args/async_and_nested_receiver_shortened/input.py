class Base:
    def items(self):
        return []


class Child(Base):
    async def items(self):
        return super(Child, self).items()

    def wrapped(self):
        def inner(self):
            return super(Child, self).items()

        return inner(self)
