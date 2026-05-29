match outer:
    case "wrap":
        match inner:
            case "alpha":
                value = 1
            case "beta":
                value = 2
            case "gamma":
                value = 3
    case "skip":
        value = 0
