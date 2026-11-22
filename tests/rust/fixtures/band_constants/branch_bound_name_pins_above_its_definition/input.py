if __debug__:
    Selector = object
else:
    Selector = None


DEFAULT = Selector


def helper():
    return 1


TABLE = dict


class Selector:
    pass
