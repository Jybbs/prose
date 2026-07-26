import os

# the tuning knob the harness injects

LIMIT = TUNING_OVERRIDE
ZEBRA = 1
APPLE = 2


def read(path):
    return os.stat(path)
