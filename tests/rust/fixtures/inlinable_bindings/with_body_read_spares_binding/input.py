def dump(path, payload):
    encoded = payload.encode()
    with path.open("wb") as handle:
        handle.write(encoded)
