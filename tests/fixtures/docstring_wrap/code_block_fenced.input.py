"""
A triple-backtick fenced code block passes through verbatim regardless of
how wide its lines run, since the fence marks the region as code rather
than prose.
"""


def example():
    """
    Summary line.

    ```python
    x = some_long_identifier + another_long_identifier + yet_another_long_identifier
    result = x * 2
    ```
    """
    return 1
