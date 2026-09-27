def dispatch(payload: Payload) -> Advice:
    return advise(payload.get("command_args") or "", payload.get("command_name", "").lstrip("/"))  # prose: skip[reflow-calls]
