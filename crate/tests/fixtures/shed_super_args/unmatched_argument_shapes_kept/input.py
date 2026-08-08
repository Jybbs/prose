class Base:
    def send(self):
        return 1


class Child(Base):
    def send(self, extra):
        return super(Child, self, **extra).send()

    def single(self):
        return super(Child).single()

    def starred(self, pair):
        return super(*pair).starred()
