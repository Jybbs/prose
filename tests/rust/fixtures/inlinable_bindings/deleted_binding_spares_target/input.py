def release():
    handle = acquire()
    use(handle)
    del handle
