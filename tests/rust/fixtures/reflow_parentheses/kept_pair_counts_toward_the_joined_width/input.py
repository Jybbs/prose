ready = (
    server.up and (cache.warm or cache.cold)
)
