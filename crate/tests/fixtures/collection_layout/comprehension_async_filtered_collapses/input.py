async def collect(source):
    return [
        item.value
        async for item in source
        if item.ready
    ]
