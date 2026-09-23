"""
Loads one module the way an import loads it and reports what it bound.

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

FIELD    = "\0"
FUNCTION = type(lambda: None)
OWN      = type.__dict__["__annotations__"].__get__
ROW      = "\x1e"
STDLIB   = "<stdlib>"
TREE     = "<tree>"
VALUE    = 1

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
        Builds the probe for the module at `located`, bound to `name`, with no
        row recorded yet.

        Args:
            located : The path the module sits at.
            name    : The dotted name an import binds it to.
        """
        self.located = located
        self.name    = name
        self.rows    = []

    def bound(self, module: object, annotated: object):
        """
        Records every name the module bound, each plain constant among them,
        and the names its annotations cover.

        Args:
            module    : The module whose namespace to read.
            annotated : The module's annotations, as reading
                        `module.__annotations__` returns them.
        """
        self.rows.append(("kind", "ok"))

        for name, value in vars(module).items():
            self.rows.append(("bound", name))

            if (spelt := constant(value)) is not None:
                self.rows.append(("const", name, spelt))

        if not isinstance(annotated, dict):
            return

        if (spelt := constant(tuple(sorted(annotated, key=str)))) is not None:
            self.rows.append(("const", "__annotations__", spelt))

    def load(self):
        """
        Imports the package holding the module, executes the module, and
        reads its annotations the way a consumer reads them, then records
        what it bound or what any step raised, beside what the run pulled in.
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
            self.unevaluated(module)

        self.rows += [
            ("loaded", held.__file__)
            for held in list(modules.values())
            if getattr(held, "__file__", None)
        ]

    def package(self):
        """
        Imports the package the module sits in, the way an import of the
        module imports its package first, so a package whose `__init__` reads
        the module finds the bound one rather than the empty module this probe
        is about to register under that name.
        """
        held, _, _ = self.name.rpartition(".")

        if held:
            __import__(held)

    def raised(self, exc: BaseException):
        """
        Records an exception, the name it turns on, the module a failed
        import named, and the frames it passed.

        Args:
            exc: The exception the module raised.
        """
        self.rows.append(("kind", "raised"))
        self.rows.append(("raise", type(exc).__name__, spelt(str(exc))))

        if named := missing(exc):
            self.rows.append(("missing", named))

        if isinstance(exc, ImportError) and exc.name:
            self.rows.append(("importing", exc.name))

        self.rows += frames(exc)

    def unevaluated(self, module: object):
        """
        Records each function and class the module defines whose annotations
        raise when read, beside the exception each raised and the name it
        turned on.

        Args:
            module: The module whose definitions to read.
        """
        for held in defined(vars(module).values(), self.name, set()):
            try:
                annotations(held)
            except BaseException as exc:
                why = (type(exc).__name__, missing(exc) or "")
                self.rows.append(("unevaluated", held.__qualname__, *why))

    def write(self, record: str):
        """
        Writes the rows as `NUL`-separated fields in `RS`-separated rows.

        Args:
            record: The path to write the record to.
        """
        with open(record, "w", encoding="utf-8") as sink:
            sink.write(ROW.join(FIELD.join(row) for row in self.rows))


def spelt(text: str) -> str:
    """
    Spells text with each tree root replaced by `TREE` and the interpreter's
    standard library directory by `STDLIB`, so a string a run derives from
    a location reads the same from either tree, across runs whose stage
    roots carry different process ids, and on any machine.

    Args:
        text: The text to spell.
    """
    for root in roots:
        text = text.replace(root, TREE)

    return text.replace(library, STDLIB)


def annotations(held: object) -> object:
    """
    Reads the annotations of a function or a class the way
    `annotationlib.get_annotations` reads them in its `VALUE` format, which
    evaluates every one. A class whose annotations descriptor raises
    `AttributeError`, as a static type's does, is read through its
    `__annotate__` where it has one and as carrying none otherwise.

    Args:
        held: The function or class to read.
    """
    if not isinstance(held, type):
        return held.__annotations__

    try:
        return OWN(held)
    except AttributeError:
        annotate = getattr(held, "__annotate__", None)

        return None if annotate is None else annotate(VALUE)


def constant(value: object) -> str | None:
    """
    Spells a value where its `repr` holds across runs, `None` otherwise. An
    `int` or `str` subclass spells through its own `repr`, so an enum member
    reads as the member. Each tree root spells as `TREE`, so a value a module
    derives from its own location reads the same from either tree.

    Args:
        value: The bound value to spell.
    """
    if value is None or isinstance(value, int | str):
        try:
            return spelt(repr(value))
        except BaseException:
            return None

    if not isinstance(value, frozenset | tuple):
        return None

    parts = [constant(item) for item in value]

    if None in parts:
        return None

    return (
        "frozenset({" + ", ".join(sorted(parts)) + "})"
        if isinstance(value, frozenset)
        else "(" + ", ".join(parts) + ("," if len(parts) == 1 else "") + ")"
    )


def defined(values, name: str, seen: set):
    """
    Yields each function and class among some values that the module `name`
    defines, then each one in the body of every such class, reading every
    value through the functions `wrapped` finds in it.

    Args:
        values : The values to read.
        name   : The dotted name of the defining module.
        seen   : The ids of the definitions already yielded.
    """
    for value in values:
        for held in wrapped(value):
            if not isinstance(held, FUNCTION | type) or id(held) in seen:
                continue

            if held.__module__ != name:
                continue

            seen.add(id(held))
            yield held

            if isinstance(held, type):
                yield from defined(vars(held).values(), name, seen)


def frames(exc: BaseException):
    """
    Yields the rows naming every frame an exception passed through.

    Args:
        exc: The exception to walk.
    """
    traceback = exc.__traceback__

    while traceback:
        yield "frame", str(traceback.tb_lineno), traceback.tb_frame.f_code.co_filename
        traceback = traceback.tb_next


def missing(exc: BaseException) -> str | None:
    """
    Returns the name an exception turns on, which is the name a failed
    `from … import …` asked for or the name a `NameError` or an
    `AttributeError` could not find, `None` where it names none.

    Args:
        exc: The exception to read.
    """
    return getattr(exc, "name_from", None) or getattr(exc, "name", None)


def wrapped(value: object) -> tuple:
    """
    Returns the functions a value holds, which are the function a
    `classmethod` or a `staticmethod` wraps, the accessors of a `property`,
    the function a `cached_property` wraps, and the value itself otherwise.

    Args:
        value: The value to read.
    """
    if isinstance(value, classmethod | staticmethod):
        return (value.__func__,)

    if isinstance(value, property):
        return (value.fget, value.fset, value.fdel)

    if type(value).__name__ == "cached_property":
        return (getattr(value, "func", None),)

    return (value,)


def main(located: str, name: str, record: str, trees: list):
    """
    Runs the module the harness named and writes the record it reads back.

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
