"""
A zero-parameter signature trips neither threshold, leaving the def
untouched. Pins the rule's no-op path on the degenerate input shape,
confirming the rewrite gates close cleanly when there is nothing to
count or measure.
"""


def render():
    return 1
