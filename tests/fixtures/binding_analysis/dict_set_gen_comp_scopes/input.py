pairs = [("a", 1), ("b", 2)]
mapping = {k: v for k, v in pairs}
keys = {k for k, _ in pairs}
total = sum(v for _, v in pairs)
