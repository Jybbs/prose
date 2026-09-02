def totals(rows):
    for row in rows:
        scaled = row.value * 2
        yield scaled + 1
