"""
Load one module the way an import loads it and report what it bound.

Usage: probe.py <record> <name> <module> <tree>...

The harness resolves the module's path and dotted name, and reads the record
back, so what stays here is the part that has to run inside the interpreter
under test.

An import here loads the interpreter's own copy into `sys.modules` ahead
of the tree's, and a module the probe pulled in can never then be seen
to break, so the probe imports nothing that is not loaded before it runs.
`importlib.util` reaches `re`, `enum`, and `typing` on 3.9, where the frozen
loaders it wraps reach nothing on any version. The record is written as
`NUL`-separated fields in `RS`-separated rows for the same reason, and the
harness bounds the run from outside, so a module that dies on a signal stays
distinguishable from one that outran its deadline.
"""

from _frozen_importlib          import module_from_spec
from _frozen_importlib_external import spec_from_file_location
from os  import _exit
from sys import argv, modules, path

FIELD = "\0"
ROW   = "\x1e"


def constant(value: object) -> "str | None":
    """
    Spell a value where it is a plain constant, and return `None` otherwise.
    """
    if value is None or isinstance(value, (int, str)):
        try:
            return repr(value)
        except BaseException:
            return None
    if not isinstance(value, (frozenset, tuple)):
        return None
    parts = [constant(item) for item in value]
    if None in parts:
        return None
    if isinstance(value, frozenset):
        return "frozenset({" + ", ".join(sorted(parts)) + "})"
    return "(" + ", ".join(parts) + ("," if len(parts) == 1 else "") + ")"


if __name__ == "__main__":

    record, name, located, *trees = argv[1:]
    path[:0] = trees

    spec          = spec_from_file_location(name, located)
    module        = module_from_spec(spec)
    modules[name] = module
    rows          = []  # prose: ignore[miscased-constants]

    try:
        spec.loader.exec_module(module)
    except BaseException as exc:
        rows.append(("kind", "raised"))
        rows.append(("error", "raises " + type(exc).__name__ + ": " + str(exc)))
        missing = getattr(exc, "name_from", None) or getattr(exc, "name", None)
        if missing is not None:
            rows.append(("missing", missing))
        traceback = exc.__traceback__
        while traceback is not None:
            where = traceback.tb_frame.f_code.co_filename
            rows.append(("frame", str(traceback.tb_lineno), where))
            traceback = traceback.tb_next
    else:
        rows.append(("kind", "ok"))
        for bound, value in vars(module).items():
            rows.append(("bound", bound))
            spelt = constant(value)
            if spelt is not None:
                rows.append(("const", bound, spelt))

    for held in list(modules.values()):
        source = getattr(held, "__file__", None)
        if source:
            rows.append(("loaded", source))

    with open(record, "w", encoding="utf-8") as sink:
        sink.write(ROW.join(FIELD.join(row) for row in rows))

    _exit(0)
