match credential.kind:
    case "apprenticeship":
        icon = "wrench"
    case "certification":
        icon = "scroll"
        notes.append("verified")
    case "program":
        icon = "cap"
    case _:
        icon = "blank"
