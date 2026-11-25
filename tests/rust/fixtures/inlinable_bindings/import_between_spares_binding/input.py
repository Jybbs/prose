def load(clock):
    start = clock.now()
    import heavy
    return heavy.size, start
