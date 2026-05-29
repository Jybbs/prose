def compute(data, factor):
    base = data * 2
    offset = base + factor
    used_twice = data + offset
    result = used_twice + offset
    return result
