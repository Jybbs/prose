def route(kind, data):
    total = sum(data)
    match kind:
        case "sum":
            return total
        case _:
            return 0
