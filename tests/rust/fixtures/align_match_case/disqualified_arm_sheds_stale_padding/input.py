match credential.kind:
    case "apprenticeship"   :
        icon = "wrench"
        notes.append("hands-on")
    case "certification"    : icon = "scroll"
    case "program"          : icon = "cap"
