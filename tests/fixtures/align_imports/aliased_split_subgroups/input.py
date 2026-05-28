"""
The shift-limit policy applies to `import M as A` runs by the same
distance rules used for `from`-import runs. A widest module that
overshoots `max-shift = 8` greedily partitions the run into
sub-groups, each aligning at its own tightened column independently
of the others.
"""

import datetime as dt
import os as o
import re as r
import collections_with_a_very_long_module_name as long
import json as j
