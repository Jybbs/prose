LABELS = {"zeta": "Z", "alpha": "A"}
WEIGHTS: dict[str, int] = {"zeta": 2, "alpha": 1}


def describe(key, keys):
    if key not in LABELS or len(LABELS) < 2:
        return LABELS.get(key, "")
    del WEIGHTS[key]
    WEIGHTS[key] = 3
    ranked = sorted(keys, key=WEIGHTS.get)
    return LABELS[key] + sorted(LABELS)[0] + {"zeta": "Z", "alpha": "A"}[ranked[0]]
