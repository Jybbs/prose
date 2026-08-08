class Base:
    def run(self):
        return 1


class Child(Base):
    def run(self, other):
        return super(Child, other).run() + super(Other, self).run()


class Other(Base):
    def run(self):
        return 2
