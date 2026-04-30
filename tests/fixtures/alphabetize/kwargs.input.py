"""
Keyword arguments at call sites alphabetize within splat-bounded
segments. A `**kwargs` splat carries no `arg` name and acts as a
hard partition, so named kwargs never cross the splat boundary
during sort. The fixture covers a call with no splat and a call
with a mid-list splat.
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
