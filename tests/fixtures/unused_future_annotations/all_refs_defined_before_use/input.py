"""
Every annotation references `Node`, which is defined at module scope
before any annotation's offset. Eager evaluation would succeed even on
Python 3.13, so the directive is removed.
"""

from __future__ import annotations


class Node:
    pass


def visit(node: Node) -> Node:
    return node


root: Node = Node()
