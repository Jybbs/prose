def load(path, fallback):
    default = fallback or {}
    try:
        return read_json(path)
    except OSError:
        return default
