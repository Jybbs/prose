"""
The rule emits no edits, so every pass leaves the source byte-for-byte
identical. A mix of flagged and preserved imports stays as written.
"""

import os
import numpy
import pandas
from os import path
