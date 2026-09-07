import zlib

# the hex helpers both writers call

import binascii


def encode(payload):
    return binascii.hexlify(payload)
