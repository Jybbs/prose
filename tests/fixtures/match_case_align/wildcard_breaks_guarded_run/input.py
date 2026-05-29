match point:
    case (x, y) if x == 0:
        label = "y_axis"
    case (x, y) if y == 0:
        label = "x_axis"
    case (x, y) if x == y:
        label = "diagonal"
    case _:
        label = "general"
