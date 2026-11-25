def factory(config):
    limit = config.limit()

    def check(value):
        return value < limit

    return check
