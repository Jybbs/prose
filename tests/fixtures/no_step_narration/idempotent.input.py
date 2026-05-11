"""
A module with no numbered-step comments produces no diagnostics. The
rule is lint-only, so any pass leaves source text unchanged.
"""


def process(payload):
    cleaned = payload.strip()
    return cleaned.lower()
