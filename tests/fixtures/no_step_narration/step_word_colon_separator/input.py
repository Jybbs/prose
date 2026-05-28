"""
A `# Step 1: text` comment matches the step-word shape with a colon
separator and earns a diagnostic.
"""


def process(payload):
    # Step 1: validate input
    if not payload:
        return None
    return payload
