"""
A single-use binding inside an `async def` is flagged the same way as
inside a regular `def`. The rule consumes the unified function-def
shape and treats both as the same scope kind.
"""


async def fetch(url):
    payload = await get(url)
    return payload
