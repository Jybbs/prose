def dispatch(payload):
    return advise(payload.get("command_args") or "", payload.get("command_name", "").lstrip("/"))
