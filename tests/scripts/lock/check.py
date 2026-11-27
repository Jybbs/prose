"""
Pins the commands `lock:check` runs and that the mise lockfile is read
through `mise lock --check`, which writes nothing.
"""

from collections.abc import Callable
from pathlib         import Path
from pytest          import MonkeyPatch, fixture
from subprocess      import CompletedProcess, run

ROOT  = Path(__file__).resolve().parents[3]
TASK  = ROOT / ".mise/tasks/lock/check"
ECHO  = 'printf \'%s %s\\n\' "${0##*/}" "$*"\n'
STOPS = ECHO + 'test "$*" != "lock --check" || test -z "${STALE:-}"\n'


@fixture
def checked(
    monkeypatch : MonkeyPatch,
    stand_in    : Callable[..., Path]
) -> Callable[..., CompletedProcess]:
    """
    Writes a stand-in `bun`, `cargo`, and `uv`, each printing its name and
    arguments on a line, and a `mise` doing the same but failing on `lock
    --check` where `STALE` is set.

    Returns:
        A function running `lock:check`, with `STALE` set where `stale` is true.
    """
    for name in ("bun", "cargo", "uv"):
        stand_in(name, ECHO)
    stand_in("mise", STOPS)

    def run_check(*, stale: bool = False) -> CompletedProcess:

        if stale:
            monkeypatch.setenv("STALE", "1")
        return run([TASK], capture_output=True, cwd=ROOT, text=True)

    return run_check


def test_lock_check_fails_where_the_mise_lockfile_is_stale(
    checked: Callable[..., CompletedProcess]
):
    """
    Pins that a failing `mise lock --check` fails `lock:check` before the
    install and the `uv` check run.
    """
    finished = checked(stale=True)

    assert finished.returncode != 0
    assert finished.stdout.splitlines()[-1] == "mise lock --check"


def test_lock_check_reads_every_recorded_platform_through_mise_lock_check(
    checked: Callable[..., CompletedProcess]
):
    """
    Pins that `lock:check` runs `mise lock --check` once between the
    `bun` and `uv` checks, leaving the lockfile to it rather than copying,
    regenerating, and restoring it, and that `cargo metadata` prints
    nothing.
    """
    finished = checked()

    assert finished.returncode == 0
    assert finished.stdout.splitlines() == [
        "bun install --dry-run --frozen-lockfile", "mise lock --check",
        "mise install --dry-run --force --locked", "uv lock --check --project crate"
    ]
