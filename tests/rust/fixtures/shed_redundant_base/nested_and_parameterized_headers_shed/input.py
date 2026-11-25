@dataclass
class Decorated(object):
    kind = "decorated"


class Generic[T] (object):
    kind = "generic"


class Outer(object):
    class Inner(object):
        kind = "inner"


def factory():
    class Local(object):
        kind = "local"

    return Local
