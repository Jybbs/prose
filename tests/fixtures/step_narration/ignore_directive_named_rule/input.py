def process(payload):
    # 1. normalize input  # prose: ignore[step-narration]
    cleaned = payload.strip()
    # 2. fold the result  # prose: ignore[align-equals]
    return cleaned.lower()
