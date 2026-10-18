def describe(exc_type, exc_traceback):
    prefix = 'Internal ' if (not exc_traceback
                                and exc_type is StopIteration) else ''
    return prefix
