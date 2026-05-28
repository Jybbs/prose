"""
A user-supplied `allow_pattern` replaces the default `^_`, so names
matching the project's local convention pass through without
flagging. Here `tmp_*` is the local prefix.
"""


def consume():
    tmp_value = compute()
    return tmp_value + 1
