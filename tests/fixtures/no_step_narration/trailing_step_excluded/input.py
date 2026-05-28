"""
An end-of-line comment that happens to match the numbered-step shape
is excluded by the own-line filter. Only own-line comments earn
diagnostics.
"""


def process(payload):
    cleaned = payload.strip()  # 1. trim whitespace
    return cleaned
