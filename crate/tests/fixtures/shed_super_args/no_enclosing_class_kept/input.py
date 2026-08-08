class Base:
    def probe(self):
        return 1


class Child(Base):
    def probe(self):
        return 2


def bind(instance):
    return super(Child, instance).probe
