from conf import SETTINGS

registry = build_registry()
database = SETTINGS["db"]
handler = registry["default"]
