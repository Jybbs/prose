def release(source):
    handle = source.open()
    del handle


def rebuild(source):
    handle = source.build()
    return handle.name
