handlers = {"open": []}
menudefs = [("file", []), ("edit", [])]


def add_entry(label):
    menudefs[0][1].append(label)


def register(event, callback):
    handlers[event][0:0] = [callback]
