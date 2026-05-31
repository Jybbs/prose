match credential.kind:
    case "apprenticeship":
        icon = "wrench"
    # legacy synonyms
    case "certification":
        icon = "scroll"
    case "program":
        icon = "cap"
