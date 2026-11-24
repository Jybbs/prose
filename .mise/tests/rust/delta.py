"""
Pins the baseline worktree `rust:delta` reads or adds, and that the baseline
it builds there stays apart from the head's build.
"""

from collections.abc import Callable
from pathlib         import Path
from pytest          import mark, param
from shutil          import rmtree
from subprocess      import CompletedProcess, run


def git(root: Path, *args: str) -> str:
    """
    Runs `git` with `args` in `root`, and returns the text it printed.
    """
    return run(
        ["git", *args],
        capture_output = True,
        check          = True,
        cwd            = root,
        text           = True
    ).stdout


@mark.parametrize("args", [param((), id="by default"), param(("main",), id="by name")])
@mark.parametrize(
    "commands",
    [
        param((), id = "branch checked out"),
        param(
            (("switch", "-q", "main"), ("restore", "-s", "feature", "crate")),
            id = "main checked out"
        )
    ]
)
def test_delta_adds_a_missing_main_worktree(
    args     : tuple[str, ...],
    commands : tuple[tuple[str, ...], ...],
    delta    : Callable[..., CompletedProcess],
    tmp_path : Path
):
    """
    Pins that a lone clone gains a detached worktree at its `main` branch
    beside it, whichever branch it has checked out, and reports the file its
    own tree formats differently.
    """
    clone = tmp_path / "clone"
    for command in commands:
        git(clone, *command)
    finished = delta("rust/delta", *args)

    assert finished.returncode == 0
    assert "module.py" in finished.stdout.splitlines()
    assert git(tmp_path / "main", "branch", "--show-current") == ""
    assert git(tmp_path / "main", "rev-parse", "HEAD") == git(
        clone,
        "rev-parse",
        "main"
    )


@mark.parametrize(
    ("command", "name"),
    [
        param(("branch", "other", "main"), "other", id = "another branch"),
        param(("branch", "-D", "main"), "main", id = "no main branch")
    ]
)
def test_delta_adds_no_worktree_away_from_the_main_branch(
    command  : tuple[str, ...],
    delta    : Callable[..., CompletedProcess],
    name     : str,
    tmp_path : Path
):
    """
    Pins that a missing baseline naming another branch, or `main` in a clone
    holding no `main` branch, stops the run and leaves no worktree behind.
    """
    git(tmp_path / "clone", *command)

    assert delta("rust/delta", name).returncode != 0
    assert not (tmp_path / name).exists()


def test_delta_builds_the_baseline_outside_a_shared_target_directory(
    delta    : Callable[..., CompletedProcess],
    tmp_path : Path
):
    """
    Pins that a `CARGO_TARGET_DIR` both trees inherit leaves the baseline a
    build of its own.
    """
    finished = delta("rust/delta", CARGO_TARGET_DIR=str(tmp_path / "shared"))

    assert finished.returncode == 0
    assert "module.py" in finished.stdout.splitlines()


def test_delta_reads_an_existing_main_worktree_as_it_stands(
    delta    : Callable[..., CompletedProcess],
    tmp_path : Path
):
    """
    Pins that a `main` worktree already beside the clone stays at the commit
    and the uncommitted edits it holds.
    """
    main = tmp_path / "main"
    git(tmp_path / "clone", "worktree", "add", "-q", "--detach", "../main", "feature")
    (main / "Cargo.toml").write_text("[profile.probe]\n\n", encoding="utf-8")
    held     = git(main, "rev-parse", "HEAD")
    finished = delta("rust/delta", "main")

    assert finished.returncode == 0
    assert "module.py" not in finished.stdout.splitlines()
    assert git(main, "rev-parse", "HEAD") == held
    assert git(main, "status", "--porcelain") == " M Cargo.toml\n?? target/\n"


def test_delta_stops_at_a_deleted_main_worktree(
    delta    : Callable[..., CompletedProcess],
    tmp_path : Path
):
    """
    Pins that a `main` worktree the clone still lists after its directory is
    deleted stops the run on git's own message, and is not added again.
    """
    git(tmp_path / "clone", "worktree", "add", "-q", "../main", "main")
    rmtree(tmp_path / "main")
    finished = delta("rust/delta", "main")

    assert finished.returncode != 0
    assert "is a missing but already registered worktree" in finished.stderr
    assert not (tmp_path / "main").exists()


def test_delta_stops_at_a_moved_main_worktree(
    delta    : Callable[..., CompletedProcess],
    tmp_path : Path
):
    """
    Pins that a `main` worktree moved from beside the clone stops the run on
    git's own message, and keeps its branch and its staged entries.
    """
    main = tmp_path / "main"
    git(tmp_path / "clone", "worktree", "add", "-q", "../main", "main")
    (main / "new.py").write_text("x = 1\n", encoding="utf-8")
    git(main, "add", "new.py")
    moved    = main.rename(tmp_path / "moved")
    finished = delta("rust/delta", "main")

    assert finished.returncode != 0
    assert "is a missing but already registered worktree" in finished.stderr
    assert not main.exists()
    assert git(moved, "branch", "--show-current") == "main\n"
    assert git(moved, "status", "--porcelain") == "A  new.py\n"
