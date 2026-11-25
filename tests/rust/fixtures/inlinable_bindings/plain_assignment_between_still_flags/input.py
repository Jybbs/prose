def summarize(rows, fallback):
    total  = sum(rows)
    label  = "done"
    marker = fallback
    return total, label, marker
