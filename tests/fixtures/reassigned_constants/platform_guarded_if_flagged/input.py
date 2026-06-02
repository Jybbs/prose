import sys

DEFAULT_BACKEND = "memory"
if sys.platform == "win32":
    DEFAULT_BACKEND = "registry"
