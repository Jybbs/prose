"""
A name matching the default `^_` allow pattern is exempt, so
`_unused` and similar conventional placeholders pass through
silently.
"""


def explainer():
    _hidden = 1
    return _hidden
