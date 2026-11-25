def kept(rows, seen):
    return [row for row in rows if not row in seen]
