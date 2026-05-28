"""
Nested multi-line containers each drop their own trailing comma.
The outer call, the inner dict, and the inner list each emit a
separate deletion edit, and the edits do not interact.
"""

posting = Posting(
    keywords=[
        "rust",
        "python",
    ],
    metadata={
        "source": "indeed",
        "tags": [
            "remote",
            "contract",
        ],
    },
)
