import sys

if sys.version_info >= (3, 11):
    from tomllib import loads
    from tomllib import dumps
    from contextlib import suppress
else:
    from tomli import loads
    from tomli import dumps
    from contextlib2 import suppress
