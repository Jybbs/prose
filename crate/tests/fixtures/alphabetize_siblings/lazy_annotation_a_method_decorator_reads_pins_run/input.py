from functools import singledispatchmethod


class Zed:
    pass


class Handler:
    @singledispatchmethod
    def show(self, value):
        return repr(value)

    @show.register
    def _(self, value: Zed):
        return "zed"


class Alpha:
    pass
