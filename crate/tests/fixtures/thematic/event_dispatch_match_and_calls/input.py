from typing import Union
import logging


def dispatch(event, registry, fallback):
    """Route an event to its handler."""
    match event.kind:
        case "create":
            return registry.create(source=event.source, actor=event.actor, timestamp=event.ts, priority=event.priority)
        case "update":
            return registry.update(source=event.source, actor=event.actor)
        case _:
            return fallback


def summarize(records: Union[list, tuple], verbose):
    "Build a compact digest of the records."
    totals = {"created": 0, "updated": 0, "deleted": 0, "skipped": 0, "failed": 0}
    return totals
