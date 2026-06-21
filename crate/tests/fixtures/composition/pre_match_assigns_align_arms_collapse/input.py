def dispatch(kind):
    x = 1
    yz = 22
    qrs = 333
    match kind:
        case "alpha":
            return x
        case "beta":
            return yz
        case _:
            return qrs
