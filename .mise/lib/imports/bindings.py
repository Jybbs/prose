"""
The names a module binds, meaning the rows of the first module-level
statement binding each name, walked into compound statements and not into a
function, class, comprehension, or lambda.
"""

from ast import (
    AST, AsyncFunctionDef, ClassDef, DictComp, ExceptHandler, FunctionDef,
    GeneratorExp, Import, ImportFrom, Lambda, ListComp, Name, SetComp, Store,
    iter_child_nodes, match_case, parse, stmt,
)
from collections.abc import Iterator
from functools       import cache
from pathlib         import Path

from diff import text_of

DEFINITIONS = (AsyncFunctionDef, ClassDef, FunctionDef)
DESCENT     = (ExceptHandler, match_case, stmt)
SCOPES      = (DictComp, GeneratorExp, Lambda, ListComp, SetComp, stmt)


@cache
def bindings(path: Path) -> dict[str, range]:
    """
    Return the rows of the first module-level statement binding each name in
    the module at `path`, one inside a compound statement counting and one
    inside a function or class not, empty where the module does not parse.
    """
    try:
        module = parse(text_of(path))
    except (SyntaxError, ValueError):
        return {}
    rows = {}
    for node in statements(module):
        for name in bound(node):
            rows.setdefault(name, header_rows(node))
    return rows


def bound(node: stmt) -> list[str]:
    """
    Return the names a module-level statement `node` binds, the name of
    a definition, the first segment of each import, and every name stored
    outside a nested scope otherwise.
    """
    match node:
        case FunctionDef() | AsyncFunctionDef() | ClassDef(): return [node.name]
        case Import() | ImportFrom():
            return [
                (alias.asname or alias.name).split(".")[0]
                for alias in node.names
                if alias.name != "*"
            ]
    return [
        child.id
        for child in own(node)
        if isinstance(child, Name) and isinstance(child.ctx, Store)
    ]


def header_rows(node: stmt) -> range:
    """
    Return the rows a statement `node` binds its names on, the header alone
    for a function or class.
    """
    if isinstance(node, DEFINITIONS):
        return range(node.lineno, max(node.body[0].lineno, node.lineno + 1))
    return range(node.lineno, node.end_lineno + 1)


def own(node: AST) -> Iterator[AST]:
    """
    Yield every node under `node` outside a nested statement, comprehension,
    or lambda.
    """
    for child in iter_child_nodes(node):
        if not isinstance(child, SCOPES):
            yield child
            yield from own(child)


def statements(node: AST) -> Iterator[stmt]:
    """
    Yield every statement under `node`, entering a compound statement, an
    exception handler, and a match case, and not a function or class.
    """
    for child in iter_child_nodes(node):
        if isinstance(child, stmt):
            yield child
        if isinstance(child, DESCENT) and not isinstance(child, DEFINITIONS):
            yield from statements(child)
