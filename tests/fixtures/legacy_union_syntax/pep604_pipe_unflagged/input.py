"""
Already-modern PEP 604 annotations using `|` are not flagged. The
walker only inspects `Subscript` shapes, so the binary `BitOr`
expression that backs `int | None` slips past entirely.
"""

x: int | None = None
y: int | str  = 0
