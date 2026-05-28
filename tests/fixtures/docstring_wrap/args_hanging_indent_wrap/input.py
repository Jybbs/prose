"""
A long Args entry wraps at the seventy six character docstring budget with
a hanging indent at the description's start column, so continuation lines
sit under the description they continue rather than at the parameter-list
indent.
"""


def configure(name, retries):
    """
    Summary line.

    Args:
        name: A descriptive name that exceeds the section budget when written out in a single line because it explains the parameter at length.
        retries: Number of retry attempts the caller wants before giving up.
    """
    return name, retries
