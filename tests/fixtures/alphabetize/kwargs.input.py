"""
Keyword arguments at call sites alphabetize into a single group.
A `**kwargs` splat carries no `arg` name and stays pinned in its
source slot. The fixture covers a multi-line call (line mode) and
the splat-pin case in one snapshot.
"""

posting = Posting(
    url="https://example.com",
    company="Cianbro",
    title="Electrician",
    date_posted=None,
    description="Wire conduits.",
)


merged = combine(
    second=2,
    first=1,
    **overrides,
    fourth=4,
    third=3,
)
