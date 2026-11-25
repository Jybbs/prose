import os


def load(path):
    return os.stat(path)


# the retry envelope the loader reads

# per-attempt ceiling, in milliseconds
ATTEMPT_MS = 250
RETRY_LIMIT = 3
TOTAL_BUDGET_MS = ATTEMPT_MS
MAX_ATTEMPTS = RETRY_LIMIT
