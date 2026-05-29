def is_ready(rec):
    return (
        rec.kind == "task"
        and rec.score > 0
        and 1 <= rec.age <= 99
        and rec.owner is None
        and rec.tag not in BLOCKED
    )
