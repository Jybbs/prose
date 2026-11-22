def guarded(mod, payload):
    expected = mod.Error
    try:
        return parse(payload)
    except expected:
        return None
