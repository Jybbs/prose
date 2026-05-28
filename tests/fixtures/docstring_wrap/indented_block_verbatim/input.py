"""
A four-space-indented block under a description paragraph passes through
verbatim because indented blocks are treated as code samples and re-flowing
them would damage their semantics.
"""


def example():
    """
    Summary line.

        x = some_long_identifier + another_long_identifier + yet_another_long_identifier
        result = x * 2
    """
    return 1
