from _pyjson import loads

try:
    from _speedups import loads
except ImportError:
    pass

value = 1
