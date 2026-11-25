def super(*args):
    return args


class Base:
    def read(self):
        return 1


class Child(Base):
    def read(self):
        return super(Child, self).read()
