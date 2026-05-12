"""
With `target-version = "3.13"` configured, trigger 2 fails (PEP 749
ships in 3.14). Trigger 3 still resolves because every annotation
references `Node`, which is module-scope-defined before any annotation
offset, and the directive is removed.
"""

from __future__ import annotations


class Node:
    pass


def visit(node: Node) -> Node:
    return node
