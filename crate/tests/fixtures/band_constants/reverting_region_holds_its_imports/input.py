import zlib
import abc


def decorate(f):
    return f


HANDLER = decorate


@HANDLER
def target():
    pass
