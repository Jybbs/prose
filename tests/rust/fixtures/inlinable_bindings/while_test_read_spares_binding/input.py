def drain(source):
    limit = compute()
    while source.next() < limit:
        source.pop()
