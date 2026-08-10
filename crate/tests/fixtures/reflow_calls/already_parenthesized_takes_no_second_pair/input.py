def bundle(entries, marker, offset, source):
    return (entries, marker, offset, source)


bundle((row for row in rows), (seen := 1), start, rows)
