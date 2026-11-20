"""
Pins the isolation `repo:test` gives each case, a base temporary directory
the session holds alone and scratch repositories git builds apart from the
repository and the configuration of the session running the suite.
"""

from collections.abc import Callable
from functools       import partial
from getpass         import getuser
from pathlib         import Path
from pytest          import MonkeyPatch, TempPathFactory, fixture
from subprocess      import run
from tempfile        import gettempdir


@fixture
def leaked(monkeypatch: MonkeyPatch):
    """
    Sets the `GIT_` variables a git hook running the suite passes on, each
    naming a path that does not exist.
    """
    for name in ("GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"):
        monkeypatch.setenv(name, "/nonexistent")


def test_scratch_builds_a_repository_apart_from_the_session(
    leaked   : None,
    scratch  : Callable[[Path], Path],
    tmp_path : Path
):
    """
    Pins that a scratch repository tracks its own files under the fixed
    author, whatever `GIT_` variables the session running the suite sets.
    """
    (tmp_path / "file").write_text("x = 1\n", encoding="utf-8")
    root = scratch(tmp_path)
    git  = partial(run, capture_output=True, check=True, cwd=root, text=True)

    assert git(["git", "ls-files"]).stdout == "file\n"
    assert git(["git", "var", "GIT_AUTHOR_IDENT"]).stdout.startswith(
        "author <author@example.com>"
    )


def test_session_writes_outside_the_shared_temporary_root(
    tmp_path_factory: TempPathFactory
):
    """
    Pins that the session's base temporary directory sits outside the
    `pytest-of-<user>` root every other session prunes.
    """
    shared = Path(gettempdir()).resolve() / f"pytest-of-{getuser()}"

    assert not tmp_path_factory.getbasetemp().is_relative_to(shared)
