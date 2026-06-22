def process(payload):
    # 1. normalize whitespace
    cleaned = payload.strip()
    # 2. lowercase the result
    folded = cleaned.lower()
    # Step 3: emit the value
    return folded
