"""
A leading `# 1. text` comment matches the numeric-dot shape and earns
a diagnostic recommending the step move into a named function.
"""


def process(payload):
    # 1. normalize whitespace
    cleaned = payload.strip()
    return cleaned
