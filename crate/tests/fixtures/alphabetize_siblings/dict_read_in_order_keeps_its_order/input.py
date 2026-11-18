LABELS = {"zeta": "Z", "alpha": "A"}
STAGES = {"checks": "Checks", "press": "Press", "deploy": "Publish"}


def first_failed(failed):
    return next((label for job, label in STAGES.items() if failed(job)), "")


def headers():
    columns = {"name": "Name", "kind": "Kind"}
    return list(columns)


def label(key):
    return LABELS[key]
