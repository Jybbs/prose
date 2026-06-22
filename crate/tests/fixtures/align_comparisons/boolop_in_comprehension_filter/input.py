filtered = [
    x
    for x in xs
    if (
        x.kind == "task"
        and x.weight == 1
    )
]
