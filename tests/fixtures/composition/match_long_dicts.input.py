"""
A `match` whose single-statement arms each return a long inline
dict.

Rules:
- collection_layout
- alphabetize
- match_case_align
- align_colons
"""


def dispatch(kind):
    match kind:
        case "alpha":
            return {"name": "alpha", "weight": 1, "extra": True, "comment_text": "primary"}
        case "beta":
            return {"name": "beta", "weight": 2, "extra": False, "comment_text": "secondary"}
        case _:
            return None
