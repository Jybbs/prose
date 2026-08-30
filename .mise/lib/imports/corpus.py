"""
Which modules of a corpus a sweep runs, meaning the interpreter owning
the corpus, the module a target narrows the run to, the entry points a run
leaves out, and the modules a format run rewrote.
"""

from filecmp    import cmp
from pathlib    import Path
from subprocess import check_output

ENTRY_POINTS = {"antigravity.py", "idlelib/idle.py", "webbrowser.py"}
ENTRY_TREES  = {"idle_test", "test", "tests", "turtledemo"}


def candidates(formatted: Path, only: str | None, original: Path) -> list[str]:
    """
    Return the modules to run, which is the one `only` names, or every
    module `formatted` rewrites beside `original` outside the entry points.
    """
    if only:
        return [only]
    return [
        relative
        for path in sorted(original.rglob("*.py"))
        if not excluded(relative := path.relative_to(original).as_posix())
        and not cmp(path, formatted / relative, shallow=False)
    ]


def excluded(relative: str) -> bool:
    """
    Report whether `relative` is an entry point rather than a library
    module.
    """
    parts = relative.split("/")
    return (
        parts[-1]   == "__main__.py"
        or relative in ENTRY_POINTS
        or not ENTRY_TREES.isdisjoint(parts[:-1])
    )


def interpreter(python: str) -> tuple[Path, str]:
    """
    Return the standard library `python` owns and its version.
    """
    try:
        stdlib, version = check_output(
            [
                python,
                "-I",
                "-c",
                "import sys, sysconfig\n"
                "print(sysconfig.get_paths()['stdlib'])\n"
                "print(sys.version.split()[0])"
            ],
            text = True
        ).splitlines()
    except OSError as error:
        raise SystemExit(f"{python or 'python'} does not run: {error}") from None
    return Path(stdlib).resolve(), version


def resolve(stdlib: Path, target: str) -> str | None:
    """
    Return the one module `target` narrows the run to, `None` for the whole
    corpus, refusing a corpus the interpreter owning `stdlib` does not.
    """
    if not target:
        return None
    path = Path(target).resolve()
    if path.is_dir():
        if path != stdlib:
            raise SystemExit(
                f"{path} is not the interpreter's standard library, {stdlib}"
            )
        return None
    if path.suffix != ".py" or not path.is_file():
        raise SystemExit(f"{target} is neither a directory nor a module")
    if not path.is_relative_to(stdlib):
        raise SystemExit(
            f"{path} is outside the interpreter's standard library, {stdlib}"
        )
    return path.relative_to(stdlib).as_posix()
