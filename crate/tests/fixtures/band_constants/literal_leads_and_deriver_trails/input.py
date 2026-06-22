import logging


def build(spec):
    return logging.getLogger(spec)


SESSION = build("app")
TIMEOUT = 30
