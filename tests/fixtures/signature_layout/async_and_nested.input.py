"""
Async and nested function definitions both walk through the rule.
The outer `async def` and the inner sync `def` each trip the count
trigger and expand at their own indents, pinning that the walker
descends recursively rather than stopping at the outer body.
"""


async def outer(target: int, palette: str, layout: tuple[int, int], spread: float, verbose: bool):
    def inner(host: str, port: int, retries: int, timeout: float, verbose: bool):
        return (host, port, retries, timeout, verbose)
    return await inner(palette, 8080, 3, 10.0, verbose)
