"""
A three-tier dependency chain reorders within each tier without
crossing tier boundaries. `RADIUS` is tier 0 (*no run-local
deps*), `DIAMETER` is tier 1 (*depends on `RADIUS`*), and
`AREA` is tier 2 (*depends on `RADIUS` through `DIAMETER`*).
Alphabetical sort applies inside each tier, so two tier-0
constants swap relative to source while staying above tier 1.
"""

RADIUS = 5
PI = 3.14
DIAMETER = RADIUS * 2
AREA = PI * (DIAMETER / 2) ** 2
