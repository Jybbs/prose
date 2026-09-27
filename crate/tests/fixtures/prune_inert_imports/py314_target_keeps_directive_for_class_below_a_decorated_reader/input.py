from __future__ import annotations

import inspect


def register(handler):
    inspect.signature(handler)
    return handler


@register
def visit(node: Node) -> Node:
    return node


class Node:
    pass
