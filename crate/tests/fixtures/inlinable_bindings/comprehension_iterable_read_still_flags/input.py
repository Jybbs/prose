def rendered(source):
    rows = source.fetch()
    return [render(row) for row in rows]
