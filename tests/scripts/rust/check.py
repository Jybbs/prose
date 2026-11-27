"""
Pins that `rust:check`, which `mise check` runs, reads the mise lockfile
beside `rustfmt`, and that its `--rustfmt` flag leaves the lockfile out.
"""

from collections.abc import Callable
from pathlib         import Path
from pytest          import MonkeyPatch, fixture, mark, param
from subprocess      import CompletedProcess, run

ROOT = Path(__file__).resolve().parents[3]
TASK = ROOT / ".mise/tasks/rust/check"


@fixture(autouse=True)
def commands(stand_in: Callable[..., Path]):
    """
    Writes a stand-in `cargo` and `mise`, each printing its name and arguments
    on a line.
    """
    for name in ("cargo", "mise"):
        stand_in(name, 'printf \'%s %s\\n\' "${0##*/}" "$*"\n')


@mark.parametrize(
    ("flag", "lines"),
    [
        param("false", ["cargo fmt --all --check", "mise lock --check"], id = "default"),
        param("true", ["cargo fmt --all --check"], id = "rustfmt only")
    ]
)
def test_check_runs_the_lockfile_check_unless_told_rustfmt_only(
    flag        : str,
    lines       : list[str],
    monkeypatch : MonkeyPatch
):
    """
    Pins that `rust:check` runs `cargo fmt --all --check` then `mise lock
    --check`, and only the first where its `--rustfmt` flag is set, as the
    `🪶 Format` row passes it.
    """
    monkeypatch.setenv("usage_rustfmt", flag)
    finished: CompletedProcess = run(
        [TASK], capture_output=True, check=True, cwd=ROOT, text=True
    )

    assert finished.stdout.splitlines() == lines
