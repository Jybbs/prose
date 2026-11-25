async def gather(rows):
    return set([row async for row in rows])
