"""
A `from os import path` carries no diagnostic because the rule
targets `Stmt::Import` rather than `Stmt::ImportFrom`. The explicit
shape the rule recommends is already in use.
"""

from os import path
