"""
A numbered-step comment inside a `# fmt: off` block still flags. The
suppression map drops auto-fix edits inside the span, but lint
diagnostics are not affected by format suppression.
"""


def process(payload):
    # fmt: off
    # 1. normalize input
    cleaned = payload.strip()
    # fmt: on
    return cleaned
