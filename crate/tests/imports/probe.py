"""
Load one module the way an import loads it and report what it bound.

Usage: probe.py <record> <name> <module> <tree>...

An import here loads the interpreter's own copy into `sys.modules` ahead of
the tree's, so the probe imports nothing that is not already loaded before
it runs.
"""

from _frozen_importlib          import module_from_spec
from _frozen_importlib_external import spec_from_file_location
from os  import _exit
from sys import argv, modules, path

FIELD = "\0"
ROW   = "\x1e"


class Probe:
    """
    One module of a tree, loaded the way an import loads it.

    Attributes:
        located : The path the module sits at.
        name    : The dotted name an import binds it to.
        rows    : The tagged rows the run has recorded so far.
    """

    def __init__(self, located: str, name: str):
        """
        Args:
            located : The path the module sits at.
            name    : The dotted name an import binds it to.
        """
        self.located = located
        self.name    = name
        self.rows    = []

    def bound(self, module: object):
        """
        Record every name the module bound and each plain constant among
        them.

        Args:
            module: The module whose namespace to read.
        """
        self.rows.append(("kind", "ok"))

        for name, value in vars(module).items():
            self.rows.append(("bound", name))

            if (spelt := constant(value)) is not None:
                self.rows.append(("const", name, spelt))

    def load(self):
        """
        Execute the module, then record what it bound and what it pulled in.
        """
        spec               = spec_from_file_location(self.name, self.located)
        module             = module_from_spec(spec)
        modules[self.name] = module

        try:
            spec.loader.exec_module(module)
        except BaseException as exc:
            self.raised(exc)
        else:
            self.bound(module)

        self.rows += [
            ("loaded", held.__file__)
            for held in list(modules.values())
            if getattr(held, "__file__", None)
        ]

    def raised(self, exc: BaseException):
        """
        Record an exception, the name it turns on, and the frames it passed.

        Args:
            exc: The exception the module raised.
        """
        self.rows.append(("kind", "raised"))
        self.rows.append(("raise", type(exc).__name__, str(exc)))

        if missing := getattr(exc, "name_from", None) or getattr(exc, "name", None):
            self.rows.append(("missing", missing))

        self.rows += frames(exc)

    def write(self, record: str):
        """
        Write the rows as `NUL`-separated fields in `RS`-separated rows.

        Args:
            record: The path to write the record to.
        """
        with open(record, "w", encoding="utf-8") as sink:
            sink.write(ROW.join(FIELD.join(row) for row in self.rows))


def constant(value: object) -> "str | None":
    """
    Spell a value where its `repr` holds across runs, `None` otherwise. An
    `int` or `str` subclass spells through its own `repr`, so an enum member
    reads as the member.

    Args:
        value: The bound value to spell.
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

    return (
        "frozenset({" + ", ".join(sorted(parts)) + "})"
        if isinstance(value, frozenset)
        else "(" + ", ".join(parts) + ("," if len(parts) == 1 else "") + ")"
    )


def frames(exc: BaseException):
    """
    The rows naming every frame an exception passed through.

    Args:
        exc: The exception to walk.
    """
    traceback = exc.__traceback__

    while traceback:
        yield "frame", str(traceback.tb_lineno), traceback.tb_frame.f_code.co_filename
        traceback = traceback.tb_next


def main(located: str, name: str, record: str, trees: list):
    """
    Run the module the harness named and write the record it reads back.

    Args:
        located : The path the module sits at.
        name    : The dotted name an import binds it to.
        record  : The path to write the record to.
        trees   : The trees to search ahead of the interpreter's own
                  library.
    """
    path[:0] = trees

    probe = Probe(located=located, name=name)
    probe.load()
    probe.write(record)

    _exit(0)


main(
    located = argv[3],
    name    = argv[2],
    record  = argv[1],
    trees   = argv[4:]
)
