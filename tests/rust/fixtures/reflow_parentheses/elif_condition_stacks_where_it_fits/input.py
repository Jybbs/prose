def pick(path, transcript, limits):
    if path is None:
        return None
    elif path != transcript and path.suffix == ".jsonl" and path.stat().st_size > limits:
        return path
    return None
