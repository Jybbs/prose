COLUMNS = {"name": "Name", "kind": "Kind"}
LABELS = {"zeta": "Z", "alpha": "A"}
STAGES = {"checks": "Checks", "press": "Press", "deploy": "Publish"}


def first_failed(failed):
    return next((label for job, label in STAGES.items() if failed(job)), "")


def headers(write):
    write(COLUMNS)
    return {"b": 2, "a": 1}


def label(key):
    return LABELS[key]
