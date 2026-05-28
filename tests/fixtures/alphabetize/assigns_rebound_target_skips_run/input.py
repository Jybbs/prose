"""
A run that re-binds the same target name skips entirely. The
second `LIMIT` shadows the first within the module, and reorder
could invert the source-defined sequence in ways the spec does
not pin.
"""

LIMIT = 10
ALPHA = 1
LIMIT = 20
