class Posting:
    __slots__ = ("title", "company", "date_posted")

    def __init__(self, company, date_posted, title):
        self.company = company
        self.date_posted = date_posted
        self.title = title
