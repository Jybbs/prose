"""
The `Returns:` section reorders alphabetically when each entry
carries a `name: description` shape. Single-paragraph `Returns:`
prose without entries falls through to the description path,
which `docstring_wrap` owns.
"""


def parse(text):
    """Split into components.

    Returns:
        tail: The trailing remainder after consuming each field.
        head: The leading token before the first separator.
        body: The mid-section captured between separators.
    """
    return text
