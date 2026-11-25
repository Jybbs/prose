finished = set(
        f for f in fs
        if f._state in [CANCELLED_AND_NOTIFIED, FINISHED])
