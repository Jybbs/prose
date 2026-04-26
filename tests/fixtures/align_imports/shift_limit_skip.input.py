"""
Same shape as `shift_limit_split` under the `skip` policy. Any
group whose widest module overshoots the cap leaves the entire
group untouched.
"""

from io import BytesIO
from re import sub
from os import getenv
from collections.abc.mapping_helpers.namespaced import OrderedDict
from sys import path
from os.path import join
