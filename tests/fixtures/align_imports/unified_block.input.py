"""
Bare aliased imports followed directly by from-imports key into
one unified block. The `as` keyword in the bares right-aligns
against the `import` keyword in the froms, so every post-keyword
name lands at one shared column.
"""

import os as o
import sys as s
from os import path
from sys import argv
