def digest(path):
    with open(path) as handle:
        data = handle.read()
    return summarize(data)
