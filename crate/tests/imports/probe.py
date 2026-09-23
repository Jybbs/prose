"""
Load one module the way an import loads it and report what it bound.

Usage: probe.py <record> <name> <module> <tree>...

An import at this level loads the interpreter's own copy into `sys.modules`
ahead of the tree's, so the probe imports nothing beyond what is already
loaded when it runs. `Probe.package` imports after `sys.path` puts the trees
first, so its import resolves against the tree, and a package that raises
ends the run there, as it ends an import of the module.
"""

from _frozen_importlib          import module_from_spec
from _frozen_importlib_external import spec_from_file_location
from os      import _exit
from os.path import dirname
from sys     import argv, modules, path

FIELD  = "\0"
ROW    = "\x1e"
STDLIB = "<stdlib>"
TREE   = "<tree>"

library = dirname(modules["os"].__file__)
roots   = []


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

    def bound(self, module: object, annotated: object):
        """
        Record every name the module bound, each plain constant among them,
        and the names its annotations cover.

        Args:
            module    : The module whose namespace to read.
            annotated : The module's annotations, as a read of the attribute
                        returns them.
        """
        self.rows.append(("kind", "ok"))

        for name, value in vars(module).items():
            self.rows.append(("bound", name))

            if (spelt := constant(value)) is not None:
                self.rows.append(("const", name, spelt))

        names = tuple(sorted(annotated, key=str)) if isinstance(annotated, dict) else None

        if (spelt := constant(names)) is not None:
            self.rows.append(("const", "__annotations__", spelt))

    def load(self):
        """
        Import the package holding the module, execute the module, and read
        its annotations the way a consumer reads them, then record what it
        bound or what any step raised, beside what the run pulled in.
        """
        spec   = spec_from_file_location(self.name, self.located)
        module = module_from_spec(spec)

        try:
            self.package()
            modules[self.name] = module
            spec.loader.exec_module(module)
            annotated = module.__annotations__
        except BaseException as exc:
            self.raised(exc)
        else:
            self.bound(module, annotated)

        self.rows += [
            ("loaded", held.__file__)
            for held in list(modules.values())
            if getattr(held, "__file__", None)
        ]

    def package(self):
        """
        Import the package the module sits in, as an import of the module
        imports it first, so a package whose `__init__` reads the module
        reaches a bound one rather than the empty module this probe is about
        to register under that name.
        """
        held, _, _ = self.name.rpartition(".")

        if held:
            __import__(held)

    def raised(self, exc: BaseException):
        """
        Record an exception, the name it turns on, the module a failed
        import read from, and the frames it passed.

        Args:
            exc: The exception the module raised.
        """
        self.rows.append(("kind", "raised"))
        self.rows.append(("raise", type(exc).__name__, spelt(str(exc))))

        if missing := getattr(exc, "name_from", None) or getattr(exc, "name", None):
            self.rows.append(("missing", missing))

        if isinstance(exc, ImportError) and exc.name:
            self.rows.append(("importing", exc.name))

        self.rows += frames(exc)

    def write(self, record: str):
        """
        Write the rows as `NUL`-separated fields in `RS`-separated rows.

        Args:
            record: The path to write the record to.
        """
        with open(record, "w", encoding="utf-8") as sink:
            sink.write(ROW.join(FIELD.join(row) for row in self.rows))


def spelt(text: str) -> str:
    """
    Spell text with each tree root replaced by `TREE` and the interpreter's
    standard library directory by `STDLIB`, so a string a run derives from
    a location reads the same from either tree, across runs whose stage
    roots carry different process ids, and on any machine.

    Args:
        text: The text to spell.
    """
    for root in roots:
        text = text.replace(root, TREE)

    return text.replace(library, STDLIB)


def constant(value: object) -> "str | None":
    """
    Spell a value where its `repr` holds across runs, `None` otherwise. An
    `int` or `str` subclass spells through its own `repr`, so an enum member
    reads as the member. Each tree root spells as `TREE`, so a value a module
    derives from its own location reads the same from either tree.

    Args:
        value: The bound value to spell.
    """
    if value is None or isinstance(value, (int, str)):
        try:
            return spelt(repr(value))
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
        trees   : The directories to search ahead of the interpreter's own
                  library, each tree beside the directory its
                  distributions are installed in.
    """
    path[:0] = trees
    roots.extend(trees)

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
