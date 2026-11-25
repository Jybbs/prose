import logging


def build(spec):
    return logging.getLogger(spec)


SESSION = build
TIMEOUT = 30
