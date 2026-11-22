"""
Pins which `tests/scripts` modules `repo:affected` runs for each change since
`origin/main`, and the arguments it hands `pytest`.
"""

from collections.abc import Callable
from pytest          import mark, param
from subprocess      import CompletedProcess

PYTEST = "uv run --locked --only-group test --project crate pytest --numprocesses=auto"
SUITE  = (
    "tests/scripts/conftest.py", "tests/scripts/py/conftest.py", "tests/scripts/py/format.py",
    "tests/scripts/py/lint.py", "tests/scripts/repo/audit.py"
)


@mark.parametrize(
    ("changed", "modules"),
    [
        param(".mise/tasks/py/format", "tests/scripts/py/format.py", id = "bash task"),
        param(
            ".mise/tasks/repo/audit.py",
            "tests/scripts/repo/audit.py",
            id = "python task"
        ),
        param("tests/scripts/py/lint.py", "tests/scripts/py/lint.py", id = "test module"),
        param("tests/scripts/py/conftest.py", "tests/scripts/py", id = "group fixture"),
        param("tests/scripts/conftest.py", "tests/scripts", id = "suite fixture"),
        param(".mise/lib/python.bash", "tests/scripts", id = "library"),
        param(".mise/config.toml", "tests/scripts", id = "tool pins"),
        param("crate/pyproject.toml", "tests/scripts", id = "test group"),
        param("crate/uv.lock", "tests/scripts", id = "test lockfile")
    ]
)
def test_affected_runs_the_modules_a_change_reaches(
    affected : Callable[..., CompletedProcess],
    changed  : str,
    modules  : str
):
    """
    Pins that a task file runs the module named for it, a test module
    runs itself, a group's fixture runs its group, and a shared fixture, a
    library, or the suite's configuration runs the whole suite.
    """
    finished = affected(changed, "edited", group="repo", tracked=SUITE)

    assert finished.returncode == 0
    assert finished.stdout == f"{PYTEST} {modules}\n"


@mark.parametrize(
    "changed",
    [
        param(".mise/tasks/rust/suite", id = "task with no module"),
        param("crate/src/lib.rs", id = "crate source"),
        param("site/index.md", id = "site")
    ]
)
def test_affected_runs_nothing_where_no_change_reaches_a_module(
    affected : Callable[..., CompletedProcess],
    changed  : str
):
    """
    Pins that a task file with no module of its own and a change outside the
    task scripts run no `pytest` and exit zero on a line saying so.
    """
    finished = affected(changed, "edited", group="repo", tracked=SUITE)

    assert finished.returncode == 0
    assert finished.stdout == "No task script or task test changed since origin/main\n"


def test_affected_hands_its_arguments_to_pytest_ahead_of_the_modules(
    affected: Callable[..., CompletedProcess]
):
    """
    Pins that the arguments the task is given reach `pytest` ahead of the
    modules it selects.
    """
    finished = affected(
        "tests/scripts/py/lint.py",
        "edited",
        "-k",
        "lint",
        group   = "repo",
        tracked = SUITE
    )

    assert finished.stdout == f"{PYTEST} -k lint tests/scripts/py/lint.py\n"


@mark.parametrize("how", ["committed", "untracked"])
def test_affected_reads_committed_and_untracked_changes(
    affected : Callable[..., CompletedProcess],
    how      : str
):
    """
    Pins that a committed change and a file git does not yet track reach the
    task, as the edit in place every other case makes does.
    """
    finished = affected("tests/scripts/py/lint.py", how, group="repo", tracked=SUITE)

    assert finished.stdout.splitlines() == [f"{PYTEST} tests/scripts/py/lint.py"]


def test_affected_runs_the_module_named_for_a_renamed_task(
    affected: Callable[..., CompletedProcess]
):
    """
    Pins that renaming a task runs the module named for the path it leaves.
    """
    finished = affected(".mise/tasks/py/format", "renamed", group="repo", tracked=SUITE)

    assert finished.stdout == f"{PYTEST} tests/scripts/py/format.py\n"


@mark.parametrize(
    ("alongside", "modules"),
    [
        param(
            ("tests/scripts/py/format.py",),
            "tests/scripts/py/format.py",
            id = "task beside its module"
        ),
        param(
            (".mise/tasks/repo/audit.py",),
            "tests/scripts/py/format.py tests/scripts/repo/audit.py",
            id = "tasks in two groups"
        ),
        param(
            (".mise/tasks/repo/audit.py", "tests/scripts/py/lint.py"),
            "tests/scripts/py/format.py tests/scripts/py/lint.py tests/scripts/repo/audit.py",
            id = "modules out of path order"
        )
    ]
)
def test_affected_runs_each_module_several_changes_reach_once(
    affected  : Callable[..., CompletedProcess],
    alongside : tuple[str, ...],
    modules   : str
):
    """
    Pins that the modules several changes reach run together, sorted, with a
    module two changes reach named once.
    """
    finished = affected(
        ".mise/tasks/py/format",
        "edited",
        alongside = alongside,
        group     = "repo",
        tracked   = SUITE
    )

    assert finished.stdout == f"{PYTEST} {modules}\n"


@mark.parametrize(
    ("changed", "expected"),
    [
        param(
            ".mise/tasks/py/format",
            f"{PYTEST} tests/scripts/py/format.py\n",
            id = "task"
        ),
        param(
            "tests/scripts/py/lint.py",
            "No task script or task test changed since origin/main\n",
            id = "test module"
        )
    ]
)
def test_affected_runs_the_modules_a_deletion_leaves(
    affected : Callable[..., CompletedProcess],
    changed  : str,
    expected : str
):
    """
    Pins that deleting a task still runs the module named for it, whereas a
    deleted module drops out of the run.
    """
    assert affected(changed, "deleted", group="repo", tracked=SUITE).stdout == expected


@mark.parametrize(
    ("reading", "modules"),
    [
        param(
            "tests/scripts/gha/brief.py",
            "tests/scripts/gha/brief.py",
            id = "test module"
        ),
        param(".mise/tasks/gha/brief.py", "tests/scripts/gha/brief.py", id = "task"),
        param("tests/scripts/gha/conftest.py", "tests/scripts/gha", id = "group fixture")
    ]
)
def test_affected_runs_what_a_file_naming_a_changed_path_reaches(
    affected : Callable[..., CompletedProcess],
    reading  : str,
    modules  : str
):
    """
    Pins that a change to a file a test module, a task, or a group's fixture
    names by its path runs the modules a change to the naming file would.
    """
    finished = affected(
        ".github/workflows/deploy.yml",
        "edited",
        group   = "repo",
        reading = (reading,),
        tracked = ("tests/scripts/gha/brief.py",)
    )

    assert finished.stdout == f"{PYTEST} {modules}\n"
