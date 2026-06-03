CONFIG = build()


class Widget:
    pass


def build(factory=Widget):
    return factory()
