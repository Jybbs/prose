def missing(key, sentinel, table):
    if not key in table:
        return table
    if not key is sentinel:
        return sentinel
    return key
