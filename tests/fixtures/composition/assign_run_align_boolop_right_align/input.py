def evaluate(record):
    name = record.get("name")
    user_id = record.get("user_id")
    age = record.get("age")
    if (
        name is None
        or user_id == 0
        or age < 18
    ):
        return None
    return record
