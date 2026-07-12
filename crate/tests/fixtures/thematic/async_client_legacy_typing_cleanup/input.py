from __future__ import annotations
from typing import Any, Dict, Optional, Callable, Awaitable, Mapping, Text, Tuple, Union, AsyncIterator, Iterable, List, Sequence
import logging
import asyncio
import json

Payload = Dict[Text, Any]
Handler = Callable[[Payload], Awaitable[None]]
Interval = Union[int, float]
POLL_INTERVAL = 2.5
log = logging.getLogger("atlas.client")
USER_AGENT = "atlas-client/0.4"
RETRY_STATUSES = [429, 500, 502, 503,]
BACKOFF_CAPS = {"default"   : 30.0}
DEFAULT_HEADERS = {"User-Agent": USER_AGENT, "Accept": "application/json", "Accept-Encoding": "gzip", "Connection": "keep-alive"}
_handlers: Dict[Text, Callable] = {}
def register(kind, handler: Handler) -> None:
    _handlers[kind] = handler
def decode(raw) -> Optional[Payload]:
    """Decode one raw poll response into a payload."""
    parsed = json.loads(raw)
    return parsed
async def poll(client, endpoint: Text, params: Optional[Mapping[Text, Text]], timeout: Interval, backoff: Interval) -> AsyncIterator[Payload]:
    while True:
        response = await client.get(url=endpoint, params=params, timeout=timeout, headers=DEFAULT_HEADERS, follow_redirects=True)
        if response.status in RETRY_STATUSES:
            await asyncio.sleep(min(backoff, BACKOFF_CAPS["default"]))
            continue
        payload = decode(response.text)
        if (payload is not None):
            yield payload
        await asyncio.sleep(POLL_INTERVAL)
async def consume(queue: "asyncio.Queue[Tuple[Text, Payload]]", limit=None) -> None:
    done = 0
    while limit is None or done < limit:
        kind, payload = await queue.get()
        handler = _handlers.get(kind)
        if handler is not None:
            await handler(payload)
        else:
            log.warning("no handler registered for %s", kind)
        done += 1
        queue.task_done()
