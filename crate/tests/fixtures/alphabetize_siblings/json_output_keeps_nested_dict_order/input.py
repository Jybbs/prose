import json

MANIFEST = {"name": "prose", "deps": {"ruff": "0.9", "insta": "1.4"}}
SETTINGS = {"width": 88, "mode": "check"}


def rows():
    return json.dumps([{"name": "a", "kind": "rule"}])


def width():
    return SETTINGS["width"]


def write(out):
    json.dump(MANIFEST, out)
