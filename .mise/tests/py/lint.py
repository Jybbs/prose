"""
Pins the files and flags `py:lint` hands the checkout's `prose check`.
"""

from collections.abc import Callable
from pytest          import mark, param
from subprocess      import CompletedProcess

FIXTURES = [
    "crate/tests/fixtures/align_equals/case/input.py",
    "site/.vitepress/tests/rules/fixtures/stand_in.py"
]


@mark.parametrize(
    ("args", "actions", "flags"),
    [
        param((), None, [], id = "local"),
        param((), "true", ["--output-format", "github"], id = "actions"),
        param(
            ("--select", "align-equals"),
            None,
            ["--select", "align-equals"],
            id = "forwarded"
        )
    ]
)
def test_lint_checks_the_tracked_python_outside_the_fixture_trees(
    actions : str | None,
    args    : tuple[str, ...],
    flags   : list[str],
    ran     : Callable[..., CompletedProcess]
):
    """
    Pins that `prose check --no-cache` receives every tracked `.py` file
    outside the fixture trees, the flag GitHub Actions annotations take when
    `GITHUB_ACTIONS` is set, and any argument the task is given, leaving out
    a file deleted from the working tree and every file of another kind.
    """
    finished = ran(
        "lint",
        ["a.py", "notes.md", "gone.py", "pkg/b.py", *FIXTURES],
        *args,
        actions = actions,
        deleted = ("gone.py",)
    )

    assert finished.returncode == 0
    assert finished.stdout.splitlines() == [
        "check", "--no-cache",
        *flags,
        "a.py", "pkg/b.py"
    ]
