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

ROOT = Path(__file__).resolve().parents[3]


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


@fixture
def sessions(tmp_path: Path) -> Callable[..., list[Path]]:
    """
    Builds a runner of pytest sessions over one case file beside the suite's
    root conftest, each taking the `temp` folder under `tmp_path` as its
    system temporary folder.

    Returns:
        A function running `case` as a session `count` times with `args`,
        which returns the base temporary directory each case reported.
    """
    (inner := tmp_path / "inner").mkdir()
    (temp := tmp_path / "temp").mkdir()
    (inner / "conftest.py").symlink_to(Path(__file__).parents[1] / "conftest.py")
    report = tmp_path / "report"

    def run_sessions(case: str, *args: str, count: int = 1) -> list[Path]:
        """
        Writes `case` beneath a reporting header and runs it as a session
        `count` times.
        """
        (inner / "case.py").write_text(
            "from os import environ\n"
            "from pytest import mark\n"
            "def report(tmp_path_factory):\n"
            "    with open(environ['REPORT'], 'a') as file:\n"
            "        print(tmp_path_factory.getbasetemp(), file=file)\n"
            f"{case}",
            encoding = "utf-8"
        )
        for _ in range(count):
            run(
                [
                    executable, "-m", "pytest",
                    *args,
                    "-o", "python_files=*.py", "-p", "no:cacheprovider", "-q"
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
        return [Path(line) for line in report.read_text(encoding="utf-8").splitlines()]

    return run_sessions


@mark.parametrize(
    "passed",
    [param(True, id = "passing session"), param(False, id = "failing session")]
)
def test_session_owns_a_root_removed_once_every_case_passed(
    passed   : bool,
    sessions : Callable[..., list[Path]],
    tmp_path : Path
):
    """
    Pins that each of two sessions writes under a base temporary directory
    of its own, removed once every case passed and kept after a failure.
    """
    roots = sessions(
        "def test_case(tmp_path_factory):\n"
        "    report(tmp_path_factory)\n"
        f"    assert {passed}\n",
        count = 2
    )

    assert len(set(roots)) == 2
    assert {root.parent for root in roots} == {(tmp_path / "temp").resolve()}
    assert [root.exists() for root in roots] == [not passed] * 2


def test_session_gives_each_worker_a_folder_under_its_own_root(
    sessions : Callable[..., list[Path]],
    tmp_path : Path
):
    """
    Pins that each worker of a session split across two writes under a
    folder of its own inside the one base temporary directory the session
    holds, removed once every case passed.
    """
    folders = set(
        sessions(
            "@mark.parametrize('case', range(4))\n"
            "def test_case(case, tmp_path_factory):\n"
            "    report(tmp_path_factory)\n",
            "--numprocesses=2"
        )
    )
    roots = {folder.parent for folder in folders}

    assert len(folders) == 2
    assert [root.parent for root in roots] == [(tmp_path / "temp").resolve()]
    assert not any(root.exists() for root in roots)


def test_session_writes_outside_the_shared_temporary_root(
    tmp_path_factory: TempPathFactory
):
    """
    Pins that the session's base temporary directory sits outside the
    `pytest-of-<user>` root every other session prunes.
    """
    shared = Path(gettempdir()).resolve() / f"pytest-of-{getuser()}"

    assert not tmp_path_factory.getbasetemp().is_relative_to(shared)


def test_task_runs_the_suite_at_the_automatic_worker_count(
    stand_in: Callable[..., Path]
):
    """
    Pins that `repo:test` runs `.mise/tests` across the worker count
    pytest-xdist detects, passing each argument it is given after the suite.
    """
    stand_in("uv")
    finished = run(
        [ROOT / ".mise/tasks/repo/test", "-k", "case"],
        capture_output = True,
        check          = True,
        cwd            = ROOT,
        text           = True
    )

    assert finished.stdout.splitlines() == [
        "run", "--locked", "--only-group", "test", "--project", "crate",
        "pytest", "--numprocesses=auto", ".mise/tests", "-k", "case"
    ]
