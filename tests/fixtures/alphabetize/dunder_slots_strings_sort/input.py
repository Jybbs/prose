"""
The string-literal items inside `__slots__` alphabetize by string
value, matching `__all__`. Tuple form parses identically through
the rule's list-or-tuple branch.
"""

class Posting:
    __slots__ = ("title", "company", "date_posted")

    def __init__(self, company, date_posted, title):
        self.company = company
        self.date_posted = date_posted
        self.title = title
