@dataclass
class Posting:
    """
    A job posting.

    Attributes:
        company: The hiring company.
        url: The listing address the class derives.
        title: The posting title.
        date_posted: The day the posting went up.
    """

    title: str
    company: str
    date_posted: str
