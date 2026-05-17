"""
A long Returns entry with the `name: description` shape wraps at the
docstring budget with a hanging indent at the description's start column,
matching the Args entry shape for named-tuple-style return docs.
"""


def lookup():
    """
    Summary line.

    Returns:
        record: The matching row from the database when the lookup succeeds, populated from every selected column and ready for the caller to consume.
        cached: Whether the record came from the in-memory cache rather than a fresh query.
    """
