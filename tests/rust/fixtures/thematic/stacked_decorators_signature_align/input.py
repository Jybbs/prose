import functools


def memoize(fn):
    return fn
def trace(fn):
    return fn

@memoize
@trace
@functools.lru_cache(maxsize=128)
def lookup(key: str, default: int = 0, ttl: float = 60.0):
    """Look up a key with optional default and TTL.
    Returns the cached value or the default."""
    return key
