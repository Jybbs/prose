"""
A multi-line function call drops its trailing comma. The argument
list's range covers `(...)`, and the backward scan from before the
closing `)` lands on the comma after the last argument.
"""

posting = Posting(
    company="Cianbro",
    date_posted=None,
    title="Electrician",
)
