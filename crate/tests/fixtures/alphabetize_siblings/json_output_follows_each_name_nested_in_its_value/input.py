import json

DEPS = {"ruff": "0.9", "insta": "1.4"}
ROW = {"name": "a", "kind": "rule"}
SETTINGS = {"width": 88, "mode": "check"}


def manifest():
    return json.dumps({"name": "prose", "deps": DEPS})


def rows():
    return json.dumps([ROW])


def width():
    return SETTINGS["width"]
