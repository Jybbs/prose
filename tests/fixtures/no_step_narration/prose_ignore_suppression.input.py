"""
A `# prose: ignore[no-step-narration]` directive on a step-narration
line suppresses the diagnostic. A directive that names a different rule
leaves the diagnostic in place.
"""


def process(payload):
    # 1. normalize input  # prose: ignore[no-step-narration]
    cleaned = payload.strip()
    # 2. fold the result  # prose: ignore[align-equals]
    return cleaned.lower()
