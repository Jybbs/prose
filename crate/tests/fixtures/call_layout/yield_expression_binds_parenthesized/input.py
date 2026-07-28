def relay(count, offset, source, total):
    return (count, offset, source, total)


def producer(stream):
    relay((yield 1), (yield from stream), start, limit)
