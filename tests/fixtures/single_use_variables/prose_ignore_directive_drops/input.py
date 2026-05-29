def basic(arg):
    x = expensive(arg)  # prose: ignore[single-use-variables]
    return x + 1
