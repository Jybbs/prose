def classify(flag, key, table, value):
    if value == None:
        return flag
    if flag == True:
        return value
    if 0 == len(table):
        return key
    if not key in table:
        return value
    return value
