import os

type Node = list[Node]
ZEBRA = 1
APPLE = 2


def read(path):
    return os.stat(path)
