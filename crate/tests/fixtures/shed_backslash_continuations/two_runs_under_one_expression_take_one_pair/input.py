def f(k, s):
    if (k in ("e", "f")) or \
         (k in ("g", "h")) \
         and not (s & ~M):
        pass
