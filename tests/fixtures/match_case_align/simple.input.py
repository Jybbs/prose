"""
A three-arm match whose every body is a single assignment. Each
arm collapses to one line and the `:` column aligns to the
widest pattern.
"""

match credential.kind:
    case "apprenticeship":
        icon = "wrench"
    case "certification":
        icon = "scroll"
    case "program":
        icon = "cap"
