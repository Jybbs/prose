async def drain(queue):
    head = queue.head
    await queue.flush()
    return head


def relay(queue, source):
    head = queue.head
    yield from source
    return head


def stream(queue):
    head = queue.head
    yield queue
    return head
