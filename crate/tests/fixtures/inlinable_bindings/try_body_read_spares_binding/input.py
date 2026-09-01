def parse(raw, fallback):
    default = fallback or {}
    try:
        return loads(raw) or default
    except ValueError:
        return {}
