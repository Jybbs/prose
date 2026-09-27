MUTATIONS = {
    "cache": {"delete"},
    "pr": {"close", "comment", "lock", "merge", "ready", "reopen", "review", "unlock"},
    **SHARED,
    "variable": {"delete", "set"},

    "a": 1,
    "bb": 2,
}
