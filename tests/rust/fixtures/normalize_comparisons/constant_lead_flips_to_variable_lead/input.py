def route(count, name):
    if "__main__" == name:
        return count
    if 0 < count:
        return name
    return 42 == count
