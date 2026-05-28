"""
Async compound shapes (`async def`, `async for`, `async with`)
recurse identically to their synchronous counterparts. Each
arm's imports flow through the same compound-statement recursion
the sync versions use, so the `is_async` flag changes nothing
about how alphabetize treats the body.
"""

async def example():
    from zeta import a
    from alpha import b

    async for item in stream:
        from gamma import c
        from beta import d

    async with session() as s:
        from omega import e
        from delta import f
