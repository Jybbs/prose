def dispatch(kind, field):
    match kind:
        case "alpha":
            return {"name": "alpha", "weight": 1, "extra": True, "comment_text": "primary"}.get(field)
        case "beta":
            return {"name": "beta", "weight": 2, "extra": False, "comment_text": "secondary"}.get(field)
        case _:
            return None
