def kept(rows, skipped):
    return [
        row for row in rows
        if row.name is not None
        and row.id  not in skipped
    ]
