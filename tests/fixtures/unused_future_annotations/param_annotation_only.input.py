"""
A function annotates its parameter but carries no return annotation.
The annotation probe still recognizes the directive as load-bearing,
and the binding-safe `Result` reference allows the directive to be
removed under trigger 3.
"""

from __future__ import annotations


class Result:
    pass


def fetch(target: Result):
    return target
