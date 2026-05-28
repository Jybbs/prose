"""
A class field whose value spans multiple source lines breaks the
colon-alignment group, so the following single-line field lands in
its own group rather than aligning across the multi-line entry.
"""


class C:
    primary: dict = {
        "alpha": 1,
        "beta": 2,
    }
    secondary: str
