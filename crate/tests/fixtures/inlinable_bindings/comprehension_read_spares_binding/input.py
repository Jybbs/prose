def scaled(values, factor):
    ratio = factor / 100
    return [value * ratio for value in values]
