# Translated by Guido van Rossum from C source provided by
# Adrian Baddeley.

from math import log as _log, exp as _exp, pi as _pi, e as _e
from math import sqrt as _sqrt, acos as _acos, cos as _cos
from os import urandom as _urandom
from _collections_abc import Sequence as _Sequence
from itertools import accumulate as _accumulate
import _random

print(_Sequence, _accumulate, _acos, _cos, _e, _exp, _log, _pi, _random, _sqrt, _urandom)
