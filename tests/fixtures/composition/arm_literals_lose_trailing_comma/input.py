def dispatch(kind):
    match kind:
        case "alpha":
            return [1, 2, 3,]
        case "beta":
            return [4, 5, 6,]
        case _:
            return []
