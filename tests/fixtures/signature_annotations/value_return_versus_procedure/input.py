def fetch(store, key):
    return store[key]


def persist(store, key, value):
    store[key] = value
