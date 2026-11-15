def evict(cache, key):
    entry = cache[key]
    del cache[key]
    return entry


def reset(counter):
    before        = counter.value
    counter.value = 0
    return before
