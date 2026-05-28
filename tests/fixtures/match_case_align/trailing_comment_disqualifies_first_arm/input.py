"""
The first arm carries a trailing comment between its `:` and
its body, so it disqualifies and stays multi-line. The two
qualifying arms after it form one sub-group and align at the
wider of their two patterns.
"""

match credential.kind:
    case "apprenticeship":  # explain
        icon = "wrench"
    case "license":
        icon = "card"
    case "degree":
        icon = "cap"
