"""
The string-literal items inside `__all__` alphabetize by string
value. Detection is by target simple name because the dunder-list
convention is what makes the list documentary; ordinary list
assignments stay untouched.
"""

__all__ = [
    "render",
    "Posting",
    "aggregate",
    "Catalog",
]


regular_list = ["b", "a", "c"]
