"""
Async definitions and nested definitions both qualify.
"""


async def outer(target, palette, layout, spread):
    def inner(host, port, retries, timeout):
        return (host, port, retries, timeout)

    return await inner(target, palette, layout, spread)
