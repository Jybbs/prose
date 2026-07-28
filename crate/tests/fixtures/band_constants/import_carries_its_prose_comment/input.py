import zlib

# the hex helpers both writers reach for

import binascii


def encode(payload):
    return binascii.hexlify(payload)
