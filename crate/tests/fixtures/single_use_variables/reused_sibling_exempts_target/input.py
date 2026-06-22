def settle(pair):
    head, tail = pair
    record(tail)
    return head + tail
