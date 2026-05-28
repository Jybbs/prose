"""
A dict whose keys mix string literals, integer literals, and bare
name references. The alignment math runs on the display width of
each key's source text, so the `:` lines up the same way regardless
of the key expression's kind.
"""

HEADER = "x-request-id"

mapping = {
    "x": 1,
    42: 2,
    HEADER: 3,
    "host": 4,
}
