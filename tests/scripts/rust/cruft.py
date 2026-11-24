"""
Pins which snapshots `rust:cruft` reports as orphans, and the runs it reads
them from.
"""

from collections.abc import Callable
from json            import dumps
from pathlib         import Path
from pytest          import fixture, mark
from shlex           import quote
from subprocess      import CompletedProcess, run

LIST = ".cache/insta/references"
READ = [
    "crate/src/snapshots/unit.snap", "tests/rust/fixtures/rule/a case/input.snap"
]
TASK = Path(__file__).resolve().parents[3] / ".mise/tasks/rust/cruft"


@fixture
def cruft(
    stand_in : Callable[..., Path],
    workdir  : Callable[[dict[str, str]], Path]
) -> Callable[..., CompletedProcess]:
    """
    Writes a stand-in `cargo` printing each command it receives, with
    `cargo metadata` printing a workspace of the one member `crate` and
    `cargo insta` appending each path in `READ`, the last first, to the list
    `INSTA_SNAPSHOT_REFERENCES_FILE` names, run from `crate` as `cargo` runs
    a test binary and spelled through `crate/..` as a test target outside
    its package's folder spells it.

    Returns:
        A function running the task in a scratch tree holding each snapshot
        in `READ` and in `beside`, where an earlier run listed each in
        `beside` as read, or none ran where `beside` is empty, and
        `cargo insta` exits with `status`.
    """
    def run_task(*beside: str, status: int = 0) -> CompletedProcess:
        """
        Writes each snapshot and the earlier list, and runs the task in the
        scratch tree.

        Returns:
            The finished run, its output captured as text.
        """
        root = workdir(dict.fromkeys([*READ, *beside], ""))
        if beside:
            workdir({LIST: "".join(f"{root}/{path}\n" for path in beside)})
        metadata = dumps(
            {
                "packages"       : [{"manifest_path": f"{root}/crate/Cargo.toml"}],
                "workspace_root" : str(root)
            }
        )
        lines = "\\n".join(f"{root}/crate/../{path}" for path in reversed(READ))
        stand_in(
            "cargo",
            f'[ "$1" = metadata ] && exec echo {quote(metadata)}\n'
            'echo "cargo $*"\n'
            '[ "$1" = insta ] || exit 0\n'
            "cd crate\n"
            f"printf '{lines}\\n' >> \"$INSTA_SNAPSHOT_REFERENCES_FILE\"\n"
            f"exit {status}\n"
        )
        return run([TASK], capture_output=True, cwd=root, text=True)

    return run_task


def test_cruft_passes_where_a_test_reads_every_snapshot(
    cruft: Callable[..., CompletedProcess]
):
    """
    Pins that the task runs the workspace without `prose_wasm` and then
    `prose_wasm`, and reports no orphan.
    """
    finished = cruft()

    assert (finished.returncode, finished.stderr) == (0, "")
    assert finished.stdout.splitlines() == [
        "cargo insta test --exclude prose_wasm --unreferenced=ignore --workspace",
        "cargo test --locked --package prose_wasm"
    ]


@mark.parametrize(
    "orphan",
    [
        "crate/src/snapshots/orphan.snap",
        "tests/rust/fixtures/rule/case/lint_findings.snap",
        "tests/rust/snapshots/a name with spaces.snap"
    ]
)
def test_cruft_rejects_a_snapshot_no_test_reads(
    cruft  : Callable[..., CompletedProcess],
    orphan : str
):
    """
    Pins that a snapshot under a workspace member or under `tests/rust/`
    that no test reads fails the task, which names that snapshot alone, even
    where an earlier run read it.
    """
    finished = cruft(orphan)

    assert finished.returncode == 1
    assert finished.stderr.splitlines() == [
        "No test reads these snapshots:",
        str((Path.cwd() / orphan).resolve())
    ]


def test_cruft_reads_no_snapshot_outside_the_rust_suites(
    cruft: Callable[..., CompletedProcess]
):
    """
    Pins that a docs-site snapshot, which Vitest reads, is no orphan.
    """
    assert cruft("tests/site/rules/__snapshots__/rule.test.ts.snap").returncode == 0


def test_cruft_stops_at_a_failing_suite(cruft: Callable[..., CompletedProcess]):
    """
    Pins that a failing suite fails the task before it reads any snapshot.
    """
    finished = cruft("crate/src/snapshots/orphan.snap", status=101)

    assert (finished.returncode, finished.stderr) == (101, "")
