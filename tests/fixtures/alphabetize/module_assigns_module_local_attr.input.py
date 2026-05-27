"""
A run whose `Attribute` root names a module-local binding written
before the run reorders normally. `SETTINGS` is module-local
(*assigned above the run*), so the attribute access does not taint
the run.
"""

SETTINGS = {"timeout": 30}

zebra = SETTINGS["timeout"]
alpha = 1
beta = 2
