# Local imports
from pkg import a
X = 1
from pkg import b

__all__ = ["b"]

print(b, X)
