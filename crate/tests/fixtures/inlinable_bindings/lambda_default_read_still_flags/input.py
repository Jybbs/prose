def joiner(parts):
    sep = ", "
    return lambda row, s=sep: s.join(row)
