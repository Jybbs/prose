"""
A class with no decorator and no recognized base class still has
its annotated field declarations alphabetized. The structural
shape (`Stmt::AnnAssign` with `Name` target) is the signal, not the
framework around it.
"""

class Posting:
    title: str = "Untitled"
    company: str
    description: str | None = None
    date_posted: str
