"""
A `# comment` line sits on its own between two arms. The
comment rides through unchanged and the arms collapse and
align around it.
"""

match credential.kind:
    case "apprenticeship":
        icon = "wrench"
    # legacy synonyms
    case "certification":
        icon = "scroll"
    case "program":
        icon = "cap"
