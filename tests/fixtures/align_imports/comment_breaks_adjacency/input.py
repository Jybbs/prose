"""
A comment between two imports (either an own-line block comment
or a trailing comment on an import line) breaks adjacency, since
the aligner's `is_line_adjacent` walks the inter-statement token
gap and aborts on any comment token. Each comment-bracketed
sub-run aligns only with its own contiguous neighbors.
"""

from collections import OrderedDict
from typing import Optional
# block comment splits the run
from sys import path
from os.path import join

import re as regex  # trailing comment splits the next run
import json as parser
