def kinds(arg):
    import os

    for value in arg:
        with open(value) as fp:
            try:
                use(arg, value, fp)
            except OSError as err:
                handle(err)
    return os
