try:
    from collections import OrderedDict
except ImportError:
    OrderedDict = dict


REGISTRY = OrderedDict


def helper():
    return 1


TABLE = dict


class OrderedDict:
    pass
