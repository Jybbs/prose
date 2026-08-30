"""
Execute one module of a tree the way an import binds it, and record what the
run left behind, meaning the namespace it bound or the exception it raised
with the name it could not find, beside every module it loaded from a tree.

Usage: execute.py <record> <module> <tree>...

Each tree goes ahead of everything else on `sys.path` in the order
given, and the module loads from the first tree carrying it, bound to the
`__name__`, `__file__`, `__package__`, and `__spec__` an import binds. The
record is the `repr` of a dict.
"""

from __future__ import annotations

import importlib.util
import sys

from os      import _exit
from os.path import dirname, exists, join

REORDERED = {"__all__"}
UNBOUND   = {
    "__builtins__", "__cached__", "__doc__", "__file__",
    "__loader__", "__path__", "__spec__"
}


def constant(value: object) -> str | None:
    """
    Return the spelling of `value` where it is a plain constant, meaning
    `None`, a `bool`, an `int`, a `str`, or a tuple or frozenset of those,
    and `None` otherwise. A frozenset spells its members sorted.
    """
    if value is None or isinstance(value, (bool, int, str)):
        return repr(value)
    if not isinstance(value, (tuple, frozenset)):
        return None
    parts = [constant(item) for item in value]
    if None in parts:
        return None
    if isinstance(value, frozenset):
        return f"frozenset({{{', '.join(sorted(parts))}}})"
    return f"({', '.join(parts)},)" if len(parts) == 1 else f"({', '.join(parts)})"


def execute(record: str, relative: str, trees: list[str]):
    """
    Load the module at `relative` from the first of `trees` carrying it,
    execute it, and write what it left behind to `record`.
    """
    sys.path[:0] = trees
    path         = next(filter(exists, (join(tree, relative) for tree in trees)))
    locations    = [dirname(path)] if relative.endswith("__init__.py") else None
    name         = module_name(relative)
    spec         = importlib.util.spec_from_file_location(
        name,
        path,
        submodule_search_locations = locations
    )
    module            = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException as exc:
        left = {
            "error"   : f"{type(exc).__name__}: {exc}",
            "frames"  : frames(exc),
            "name"    : missing(exc),
            "outcome" : "raised"
        }
    else:
        names, constants = namespace(module)
        left             = {"constants": constants, "names": names, "outcome": "ok"}
    left["loaded"] = loaded(trees)
    with open(record, "w", encoding="utf-8") as sink:
        sink.write(repr(left))


def frames(exc: BaseException) -> list[tuple[str, int]]:
    """
    Return the `(file, line)` of every frame the exception chain passed
    through, in the order a traceback prints them.
    """
    chain = []
    while exc is not None and all(link is not exc for link in chain):
        chain.append(exc)
        exc = exc.__cause__ or (None if exc.__suppress_context__ else exc.__context__)
    seen = []
    for link in reversed(chain):
        tb = link.__traceback__
        while tb is not None:
            seen.append((tb.tb_frame.f_code.co_filename, tb.tb_lineno))
            tb = tb.tb_next
    return seen


def loaded(trees: list[str]) -> list[str]:
    """
    Return the path, relative to its tree, of every module `sys.modules`
    holds from one of `trees`, sorted.
    """
    modules = list(sys.modules.values())
    files   = [getattr(module, "__file__", None) or "" for module in modules]
    found   = set()
    for tree in trees:
        prefix = f"{tree}/"
        found |= {file[len(prefix):] for file in files if file.startswith(prefix)}
    return sorted(found)


def missing(exc: BaseException) -> str | None:
    """
    Return the name a `NameError` or `AttributeError` says is unbound, the
    name an `ImportError` could not import or the module it could not find,
    or `None`.
    """
    return getattr(exc, "name_from", None) or getattr(exc, "name", None)


def module_name(relative: str) -> str:
    """
    Return the dotted name an import binds the module at `relative` to.
    """
    parts = relative.removesuffix(".py").split("/")
    if parts[-1] == "__init__":
        parts.pop()
    return ".".join(parts)


def namespace(module: object) -> tuple[list[str], dict[str, str]]:
    """
    Return the names `module` binds beyond the loader's own and its
    docstring, sorted, and the plain constants among them, spelt, the value
    of a name the formatter reorders left out.
    """
    bound = {key: value for key, value in vars(module).items() if key not in UNBOUND}
    spelt = {}
    for key, value in bound.items():
        if key not in REORDERED and (spelling := constant(value)):
            spelt[key] = spelling
    return sorted(bound), spelt


if __name__ == "__main__":

    record, relative, *trees = sys.argv[1:]
    execute(record, relative, trees)
    _exit(0)
