"""
A compound `import` flags every segment outside the allowlist while
leaving the allowlisted ones alone. `os` and `sys` fire, `numpy` is
preserved, all on the same statement.
"""

import os, sys, numpy
