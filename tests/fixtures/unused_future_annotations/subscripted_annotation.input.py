"""
A subscripted generic annotation routes `any_over_expr` through the
`Subscript` into each inner `Name`. With both `Container` and `Item`
defined above the annotation, trigger 3 fires and the directive is
removed.
"""

from __future__ import annotations


class Container:
    pass


class Item:
    pass


x: Container[Item] = Container()
