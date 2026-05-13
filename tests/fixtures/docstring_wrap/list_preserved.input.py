"""
List items pass through verbatim. A long bulleted item is not re-flowed
onto multiple lines because the list shape carries meaning the wrap math
must not erase.
"""


def example():
    """
    The supported markers cover the conventional set:

    - dash markers introduce an unordered list item that may run as long as the author wants without being split
    - star markers behave the same way under the same line budget
    + plus markers also work, mirroring the CommonMark allowed marker set
    1. numeric ordered items are recognized as list items too

    Continuation prose after the list resumes the description budget.
    """
    return 1
