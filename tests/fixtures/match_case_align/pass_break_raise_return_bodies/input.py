def dispatch(token):
    match token:
        case "noop":
            pass
        case "skip":
            continue
        case "stop":
            break
        case "boom":
            raise RuntimeError("boom")
        case "echo":
            log(token)
        case _:
            return None
