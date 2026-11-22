def dispatch(kind):
    match kind:
        case "alpha":
            return {"name": "alpha", "weight": 1, "extra_field": True, "comment_text": "primary"}
        case "beta":
            return {"name": "beta", "weight": 2, "extra_field": False, "comment_text": "secondary"}
        case _:
            return None
