"""
A run of consecutive `import M as A` statements aligns the `as`
keyword across them. The widest module name fixes the shared
column where every `as` lands.
"""

import collections as c
import datetime as dt
import functools as fn
import itertools as it
