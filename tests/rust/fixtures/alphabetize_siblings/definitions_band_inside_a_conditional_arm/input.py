import sys

if sys.version_info >= (3, 12):
    def render():
        pass

    class Widget:
        pass

    def build():
        pass
else:
    def render():
        pass
