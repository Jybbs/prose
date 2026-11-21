"""
Pins the isolation `repo:test` gives each case, a base temporary directory
the session holds alone and removes once every case passed, and scratch
repositories git builds apart from the repository and the configuration of
the session running the suite.
"""

from collections.abc import Callable
from functools       import partial
from getpass         import getuser
from os              import environ
from pathlib         import Path
from pytest          import MonkeyPatch, TempPathFactory, fixture, mark, param
from subprocess      import run
from sys             import executable
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


def test_scratch_reads_no_configuration_but_its_own(
    monkeypatch : MonkeyPatch,
    scratch     : Callable[[Path], Path],
    tmp_path    : Path
):
    """
    Pins that git in a scratch repository reads its local configuration
    alone, whatever global configuration the session running the suite
    holds.
    """
    (home := tmp_path / "home").mkdir()
    (home / ".gitconfig").write_text("[user]\n\tname = leaked\n", encoding="utf-8")
    monkeypatch.setenv("HOME", str(home))
    (root := tmp_path / "repo").mkdir()
    listed = run(
        ["git", "config", "--list", "--name-only", "--show-scope"],
        capture_output = True,
        check          = True,
        cwd            = scratch(root),
        text           = True
    )

    assert {line.split()[0] for line in listed.stdout.splitlines()} == {"local"}


@mark.parametrize(
    "passed",
    [param(True, id = "passing session"), param(False, id = "failing session")]
)
def test_session_owns_a_root_removed_once_every_case_passed(
    passed   : bool,
    tmp_path : Path
):
    """
    Pins that each of two sessions writes under a base temporary directory
    of its own, removed once every case passed and kept after a failure.
    """
    (inner := tmp_path / "inner").mkdir()
    (temp := tmp_path / "temp").mkdir()
    (inner / "conftest.py").symlink_to(Path(__file__).parents[1] / "conftest.py")
    (inner / "case.py").write_text(
        "from os import environ\n"
        "def test_case(tmp_path_factory):\n"
        "    with open(environ['REPORT'], 'a') as file:\n"
        "        print(tmp_path_factory.getbasetemp(), file=file)\n"
        f"    assert {passed}\n",
        encoding = "utf-8"
    )
    report = tmp_path / "report"
    for _ in range(2):
        run(
            [
                executable, "-m", "pytest", "-o",
                "python_files=*.py", "-p", "no:cacheprovider", "-q"
            ],
            capture_output = True,
            cwd            = inner,
            env            = {
                **environ,
                "PYTEST_ADDOPTS" : "",
                "REPORT"         : str(report),
                "TMPDIR"         : str(temp)
            }
        )
    roots = [Path(line) for line in report.read_text(encoding="utf-8").splitlines()]

    assert len(set(roots)) == 2
    assert {root.parent for root in roots} == {temp.resolve()}
    assert [root.exists() for root in roots] == [not passed] * 2


def test_session_gives_each_worker_a_folder_under_its_own_root(tmp_path: Path):
    """
    Pins that each worker of a session split across two writes under a
    folder of its own inside the one base temporary directory the session
    holds, removed once every case passed.
    """
    (inner := tmp_path / "inner").mkdir()
    (temp := tmp_path / "temp").mkdir()
    (inner / "conftest.py").symlink_to(Path(__file__).parents[1] / "conftest.py")
    (inner / "case.py").write_text(
        "from os import environ\n"
        "from pytest import mark\n"
        "@mark.parametrize('case', range(4))\n"
        "def test_case(case, tmp_path_factory):\n"
        "    with open(environ['REPORT'], 'a') as file:\n"
        "        print(tmp_path_factory.getbasetemp(), file=file)\n",
        encoding = "utf-8"
    )
    report = tmp_path / "report"
    run(
        [
            executable, "-m", "pytest", "--numprocesses=2", "-o",
            "python_files=*.py", "-p", "no:cacheprovider", "-q"
        ],
        capture_output = True,
        check          = True,
        cwd            = inner,
        env            = {
            **environ,
            "PYTEST_ADDOPTS" : "",
            "REPORT"         : str(report),
            "TMPDIR"         : str(temp)
        }
    )
    folders = {Path(line) for line in report.read_text(encoding="utf-8").splitlines()}

    assert len(folders) == 2
    assert {folder.parent.parent for folder in folders} == {temp.resolve()}
    assert len({folder.parent for folder in folders}) == 1
    assert not any(folder.parent.exists() for folder in folders)


def test_session_writes_outside_the_shared_temporary_root(
    tmp_path_factory: TempPathFactory
):
    """
    Pins that the session's base temporary directory sits outside the
    `pytest-of-<user>` root every other session prunes.
    """
    shared = Path(gettempdir()).resolve() / f"pytest-of-{getuser()}"

    assert not tmp_path_factory.getbasetemp().is_relative_to(shared)
