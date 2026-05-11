"""
A `# fmt: off` block carries the same suppression weight against
lint diagnostics as it does against edits. The numbered-step comment
inside the block escapes the rule's emission path.
"""


def process(payload):
    # fmt: off
    # 1. normalize input
    cleaned = payload.strip()
    # fmt: on
    return cleaned
