def dispatch(event):
    match event.kind:
        case "create": return Counter(timestamp=event.ts, source=event.src, action="create")
        case "update": return Counter(timestamp=event.ts, source=event.src, action="update")
        case "delete": return Counter(timestamp=event.ts, source=event.src, action="delete")
        case _      : return None


SETTINGS = {
    "default_action" : "noop",
}


def required_only(name):
    return {name,}
