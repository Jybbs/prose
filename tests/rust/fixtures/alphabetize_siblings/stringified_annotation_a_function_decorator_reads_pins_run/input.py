from __future__ import annotations
from functools import singledispatch


@singledispatch
def show(value):
    return repr(value)


class Zed:
    pass


@show.register
def _(value: Zed):
    return "zed"


class Alpha:
    pass
