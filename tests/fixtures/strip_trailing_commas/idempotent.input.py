"""
A container that already lacks a trailing comma round-trips
unchanged. The backward scan finds the last element rather than a
comma and emits no edit.
"""

posting = Posting(
    company="Cianbro",
    title="Electrician"
)
