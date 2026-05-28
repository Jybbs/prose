"""
Multi-name `import` statements (comma-separated, with or without
aliases per name) skip alignment entirely. The rule aligns the
unique `as` keyword in single-aliased imports, whereas multi-name
shapes have no single anchor and so do not qualify.
"""

import collections as c
import os, sys
import re as regex, json as parser
import datetime as dt
