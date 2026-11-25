import logging


class Configure:
    logging.basicConfig(level=logging.DEBUG)
    seen = logging.root.level


LEVEL = logging.root.level


def helper():
    return 1


TABLE = dict
