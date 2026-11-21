"""
Pins the version `repo:bump` moves `crate/Cargo.toml` and the `prose` entry
of `Cargo.lock` to for each part, that every other byte of both files stays
as it stood, and the inputs it refuses before writing either file.
"""

from collections.abc import Callable
from pathlib         import Path
from pytest          import MonkeyPatch, fixture, mark, param, raises
from types           import ModuleType

LOCKFILE = """version = 4

[[package]]
name = "other"
version = "{0}"

[[package]]
name = "prose"
version = "{0}"
dependencies = [
 "other",
]
"""
MANIFEST = """[package]
edition                = "2024"
name                   = "prose"
version                = "{0}"

[package.metadata.ruff]
release = "{0}"
"""


@fixture
def bump(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Loads the `repo:bump` script.
    """
    return task("repo/bump")


@fixture
def tree(workdir: Callable[[dict[str, str]], Path]) -> Path:
    """
    Writes a working tree whose manifest and lockfile both carry `0.11.0`,
    beside an unrelated `0.11.0` in each, and makes it the current directory.
    """
    return workdir(
        {"Cargo.lock": LOCKFILE.format("0.11.0"), "crate/Cargo.toml": MANIFEST.format("0.11.0")}
    )


@mark.parametrize(
    ("part", "version"),
    [
        param("patch", "0.11.1", id = "patch"),
        param("minor", "0.12.0", id = "minor"),
        param("major", "1.0.0", id = "major")
    ]
)
def test_main_moves_the_manifest_and_the_lock_entry(
    bump        : ModuleType,
    monkeypatch : MonkeyPatch,
    part        : str,
    tree        : Path,
    version     : str
):
    """
    Pins that each part raises its own place and zeroes those after it, in
    the manifest's `[package].version` and the lockfile's `prose` entry
    alone, keeping the manifest's aligned `=` and every other line.
    """
    monkeypatch.setattr("sys.argv", ["bump", part])

    bump.main()

    assert (tree / "crate/Cargo.toml").read_text(encoding="utf-8") == MANIFEST.format(
        "0.11.0"
    ).replace('version                = "0.11.0"', f'version                = "{version}"')
    assert (tree / "Cargo.lock").read_text(encoding="utf-8") == LOCKFILE.format(
        "0.11.0"
    ).replace('"prose"\nversion = "0.11.0"', f'"prose"\nversion = "{version}"')


@mark.parametrize(
    ("files", "message"),
    [
        param(
            {
                "Cargo.lock"       : LOCKFILE.format("0.11.0-rc.1"),
                "crate/Cargo.toml" : MANIFEST.format("0.11.0-rc.1")
            },
            "crate/Cargo.toml carries 0.11.0-rc.1, not three numbers",
            id = "prerelease"
        ),
        param(
            {"Cargo.lock": LOCKFILE.format("0.10.2")},
            "Cargo.lock locks prose at 0.10.2, not 0.11.0",
            id = "lagging lock"
        )
    ]
)
def test_main_writes_nothing_on_a_refused_input(
    bump        : ModuleType,
    files       : dict[str, str],
    message     : str,
    monkeypatch : MonkeyPatch,
    tree        : Path,
    workdir     : Callable[[dict[str, str]], Path]
):
    """
    Pins that a version other than three numbers, and a lockfile whose
    `prose` entry differs from the manifest, stop the task before it writes
    either file.
    """
    written = {
        path: (tree / path).read_text(encoding="utf-8")
        for path in ("Cargo.lock", "crate/Cargo.toml")
    } | files
    workdir(files)
    monkeypatch.setattr("sys.argv", ["bump", "patch"])

    with raises(SystemExit) as stopped:
        bump.main()

    assert stopped.value.code == message
    assert {path: (tree / path).read_text(encoding="utf-8") for path in written} == written


def test_main_rejects_an_unknown_part(bump: ModuleType, monkeypatch: MonkeyPatch, tree: Path):
    """
    Pins that a part other than `major`, `minor`, or `patch` exits 2 through
    the parser before either file is read.
    """
    monkeypatch.setattr("sys.argv", ["bump", "micro"])

    with raises(SystemExit) as stopped:
        bump.main()

    assert stopped.value.code == 2
