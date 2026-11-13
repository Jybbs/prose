"""
Pins that `rust:proof` starts both proof binaries before either finishes,
prints each binary's exit status and its output in a block of its own, fails
once either binary fails, and stops both binaries on a signal, leaving no
log directory behind.
"""

from os         import environ, kill, killpg
from pathlib    import Path
from pytest     import mark, param
from signal     import SIGINT, SIGTERM, Signals
from subprocess import PIPE, Popen, run
from time       import monotonic, sleep

ROOT     = Path(__file__).resolve().parents[3]
STAND_IN = "#!/bin/sh\n" + """\
case " $* " in *" --no-run "*) exit 0 ;; esac
for arg; do case $arg in --test=*) name=${arg#--test=} ;; esac; done
echo $$ > "$RENDEZVOUS/$name"
n=0
until [ -s "$RENDEZVOUS/corpus" ] && [ -s "$RENDEZVOUS/settle" ]; do
  n=$((n + 1))
  [ "$n" -gt 200 ] && { echo "$name ran alone"; exit 3; }
  sleep 0.05
done
echo "$name finding"
[ -n "${HOLD:-}" ] && exec sleep 30
eval "exit \\${EXIT_$name:-0}"
"""


def scratch(tmp_path: Path, **extra: str) -> dict[str, str]:
    """
    Returns the environment running the task with the stand-in `cargo` first
    on `PATH`, its rendezvous and temporary directories under `tmp_path`,
    and each variable in `extra`.
    """
    for name in ("bin", "rendezvous", "tmp"):
        (tmp_path / name).mkdir()
    (cargo := tmp_path / "bin/cargo").write_text(STAND_IN, encoding="utf-8")
    cargo.chmod(0o755)

    return {
        **environ,
        **extra,
        "MISE_PROJECT_ROOT" : str(ROOT),
        "PATH"              : f"{cargo.parent}:{environ['PATH']}",
        "RENDEZVOUS"        : str(tmp_path / "rendezvous"),
        "TMPDIR"            : str(tmp_path / "tmp")
    }


@mark.parametrize(
    ("exits", "returncode"),
    [
        param({}, 0, id = "both-pass"),
        param({"EXIT_corpus": "101"}, 1, id = "corpus-fails"),
        param({"EXIT_settle": "101"}, 1, id = "settle-fails")
    ]
)
def test_proof_runs_both_binaries_side_by_side_and_reports_each(
    exits      : dict[str, str],
    returncode : int,
    tmp_path   : Path
):
    """
    Pins that each binary reaches the other's start before it finishes, that
    an exit status line names each binary, that each binary's output follows
    a heading of its own, that the task fails once either binary fails, and
    that the log directory is gone once the task exits.
    """
    finished = run(
        [ROOT / ".mise/tasks/rust/proof"],
        capture_output = True,
        env            = scratch(tmp_path, **exits),
        text           = True
    )

    assert finished.returncode == returncode
    for name in ("corpus", "settle"):
        status = exits.get(f"EXIT_{name}", "0")
        assert f"{name} exited {status} after " in finished.stdout
        assert f"\n{name} output\n\n{name} finding\n" in finished.stdout
        assert finished.stdout.count(f"{name} finding") == 1
    assert not any((tmp_path / "tmp").iterdir())


@mark.parametrize(
    ("signal", "returncode"),
    [
        param(SIGINT, 130, id = "interrupt"),
        param(SIGTERM, 143, id = "terminate")
    ]
)
def test_proof_stops_both_binaries_on_a_signal(
    returncode : int,
    signal     : Signals,
    tmp_path   : Path
):
    """
    Pins that a signal sent to the task's process group once both binaries
    run stops both of them, prints the output each wrote so far in its own
    block, exits with the signal's status, and removes the log directory.
    """
    rendezvous = tmp_path / "rendezvous"
    task       = Popen(
        [ROOT / ".mise/tasks/rust/proof"],
        env               = scratch(tmp_path, HOLD="1"),
        start_new_session = True,
        stdout            = PIPE,
        text              = True
    )
    deadline = monotonic() + 10
    pids     = [rendezvous / name for name in ("corpus", "settle")]
    while not all(pid.is_file() and pid.stat().st_size for pid in pids):
        assert monotonic() < deadline
        sleep(0.01)
    killpg(task.pid, signal)
    stdout, _ = task.communicate(timeout=10)

    assert task.returncode == returncode
    for name in ("corpus", "settle"):
        assert stdout.count(f"\n{name} output\n\n") == 1
        pid = int((rendezvous / name).read_text(encoding="utf-8"))
        try:
            kill(pid, 0)
        except ProcessLookupError:
            continue
        raise AssertionError(f"{name} stand-in {pid} still runs")
    assert not any((tmp_path / "tmp").iterdir())
