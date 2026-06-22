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
