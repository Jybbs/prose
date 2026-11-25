def close(path, fallback):
    default = fallback or {}
    try:
        return read(path)
    finally:
        record(default)
