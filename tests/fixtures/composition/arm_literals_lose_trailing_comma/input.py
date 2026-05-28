"""
A `match` whose single-statement arms each return an inline
collection literal carrying a trailing comma.

Rules:
- strip_trailing_commas
- match_case_align
"""


def dispatch(kind):
    match kind:
        case "alpha":
            return [1, 2, 3,]
        case "beta":
            return [4, 5, 6,]
        case _:
            return []
