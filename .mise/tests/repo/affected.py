"""
Pins which `.mise/tests` modules `repo:affected` runs for each change since
`origin/main`, and the arguments it hands `pytest`.
"""

from collections.abc import Callable
from pytest          import mark, param
from subprocess      import CompletedProcess

PYTEST = "uv run --locked --only-group test --project crate pytest"
SUITE = (
    ".mise/tests/conftest.py",
    ".mise/tests/py/conftest.py",
    ".mise/tests/py/format.py",
    ".mise/tests/py/lint.py",
    ".mise/tests/repo/audit.py"
)


@mark.parametrize(
    ("changed", "modules"),
    [
        param(".mise/tasks/py/format", ".mise/tests/py/format.py", id = "bash task"),
        param(".mise/tasks/repo/audit.py", ".mise/tests/repo/audit.py", id = "python task"),
        param(".mise/tests/py/lint.py", ".mise/tests/py/lint.py", id = "test module"),
        param(".mise/tests/py/conftest.py", ".mise/tests/py", id = "group fixture"),
        param(".mise/tests/conftest.py", ".mise/tests", id = "suite fixture"),
        param(".mise/tests/pytest.toml", ".mise/tests", id = "suite config"),
        param(".mise/lib/python.bash", ".mise/tests", id = "library"),
        param("crate/pyproject.toml", ".mise/tests", id = "test group"),
        param("crate/uv.lock", ".mise/tests", id = "test lockfile")
    ]
)
def test_affected_runs_the_modules_a_change_reaches(
    affected : Callable[..., CompletedProcess],
    changed  : str,
    modules  : str
):
    """
    Pins that a task file runs the module named for it, a test module runs
    itself, a group's fixture runs its group, and a shared fixture, a
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
    Pins that a task file with no module of its own and a change outside
    the task scripts run no `pytest` and exit zero on a line saying so.
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
        ".mise/tests/py/lint.py", "edited", "-k", "lint", group="repo", tracked=SUITE
    )

    assert finished.stdout == f"{PYTEST} -k lint .mise/tests/py/lint.py\n"


@mark.parametrize("how", ["committed", "untracked"])
def test_affected_reads_committed_and_untracked_changes(
    affected : Callable[..., CompletedProcess],
    how      : str
):
    """
    Pins that a committed change and a file git does not yet track reach
    the task, as the edit in place every other case makes does.
    """
    finished = affected(".mise/tests/py/lint.py", how, group="repo", tracked=SUITE)

    assert finished.stdout.splitlines() == [f"{PYTEST} .mise/tests/py/lint.py"]


@mark.parametrize(
    ("alongside", "modules"),
    [
        param(
            (".mise/tests/py/format.py",),
            ".mise/tests/py/format.py",
            id = "task beside its module"
        ),
        param(
            (".mise/tasks/repo/audit.py",),
            ".mise/tests/py/format.py .mise/tests/repo/audit.py",
            id = "tasks in two groups"
        )
    ]
)
def test_affected_runs_each_module_several_changes_reach_once(
    affected  : Callable[..., CompletedProcess],
    alongside : tuple[str, ...],
    modules   : str
):
    """
    Pins that the modules several changes reach run together, sorted, with
    a module two changes reach named once.
    """
    finished = affected(
        ".mise/tasks/py/format", "edited", alongside=alongside, group="repo", tracked=SUITE
    )

    assert finished.stdout == f"{PYTEST} {modules}\n"


@mark.parametrize(
    ("changed", "expected"),
    [
        param(
            ".mise/tasks/py/format",
            f"{PYTEST} .mise/tests/py/format.py\n",
            id = "task"
        ),
        param(
            ".mise/tests/py/lint.py",
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
    finished = affected(changed, "deleted", group="repo", tracked=SUITE)

    assert finished.stdout == expected
