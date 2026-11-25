HANDLERS = {
    "cache": clear(),
    "pr": dispatch(close, comment, lock, merge, ready, reopen, review, unlock, abc),
    "variable": assign(delete, set),
}  # prose: skip[reflow-calls]
