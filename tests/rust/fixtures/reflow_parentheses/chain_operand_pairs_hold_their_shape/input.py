def check(flags):
    if (flags.alpha or flags.beta) and (flags.gamma or flags.delta):
        return
