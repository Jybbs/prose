def dispatch(kind):
    match kind:
        case "alpha":
            return {"name": "alpha", "weight": 1, "extra": True, "comment_text": "primary"}
        case "beta":
            return {"name": "beta", "weight": 2, "extra": False, "comment_text": "secondary"}
        case _:
            return None
