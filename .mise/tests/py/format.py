"""
Pins the files and flags `py:format` hands the checkout's `prose format`.
"""

from collections.abc import Callable
from subprocess      import CompletedProcess


def test_format_rewrites_the_tracked_python_outside_the_fixture_trees(
    ran: Callable[..., CompletedProcess]
):
    """
    Pins that `prose format --no-cache` receives every tracked `.py` file
    outside the fixture trees, with any argument the task is given ahead of
    the files, and no flag for GitHub Actions annotations.
    """
    finished = ran(
        "format",
        ["a.py", "crate/tests/fixtures/align_equals/case/input.py"],
        "--diff",
        actions = "true"
    )

    assert finished.returncode == 0
    assert finished.stdout.splitlines() == ["format", "--no-cache", "--diff", "a.py"]
