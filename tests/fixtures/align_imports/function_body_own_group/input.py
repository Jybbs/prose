from collections import OrderedDict
from typing import Optional


def lazy_setup():
    from sys import path
    from os.path import join
    from contextlib import suppress

    return path, join, suppress
