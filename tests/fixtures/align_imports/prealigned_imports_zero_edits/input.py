"""
Input that is already aligned. Each gap already carries the
target width of spaces, so the rule emits zero edits and the
output equals the input byte-for-byte.
"""

from collections import OrderedDict
from typing      import Optional
from sys         import path

import collections as c
import datetime    as dt
import functools   as fn
