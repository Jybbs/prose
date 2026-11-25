"""
Pins that `rust:probe` and the `sweep_test` the corpus sweeps run each build
over the workspace without `prose_wasm`, the package selection `rust:proof`
builds over, and that the delta, mutation, and bake rows build their
binaries over the packages `rust:probe` builds its binaries over.
"""

from collections.abc import Callable
from json            import dumps
from pathlib         import Path
from pytest          import MonkeyPatch, mark, param
from shlex           import quote
from subprocess      import CompletedProcess, run

ROOT      = Path(__file__).resolve().parents[3]
SELECTION = "build --locked -p prose -p prose_corpus"


def sourced(
    call       : str,
    stand_in   : Callable[..., Path],
    executable : str | None = "true"
) -> CompletedProcess[str]:
    """
    Runs `call` from the checkout after sourcing the corpus and delta
    libraries, against a stand-in `cargo` that prints its arguments to
    stderr and reports `executable` as the binary each call names.

    Returns:
        The finished run, its output captured as text.
    """
    stand_in(
        "cargo",
        "printf '%s\\n' \"$*\" >&2\n"
        "printf '{\"executable\":%s,\"target\":{\"name\":\"%s\"}}\\n' "
        f'{quote(dumps(executable))} "$8"\n'
    )
    script = (
        f'source "{ROOT}/.mise/lib/corpus.bash"; '
        f'source "{ROOT}/.mise/lib/delta.bash"; {call}'
    )
    return run(
        ["bash", "-euo", "pipefail", "-c", script],
        capture_output = True,
        cwd            = ROOT,
        text           = True
    )


def test_built_binary_builds_release_where_the_tree_lacks_the_profile(
    monkeypatch : MonkeyPatch,
    stand_in    : Callable[..., Path]
):
    """
    Pins that `built_binary` in a tree defining no profile named by
    `PROSE_DELTA_PROFILE` says so and builds the release profile over the
    packages `rust:probe` builds its binaries over.
    """
    monkeypatch.setenv("PROSE_DELTA_PROFILE", "absent")
    finished = sourced("built_binary", stand_in)

    assert (finished.returncode, finished.stderr.splitlines()) == (
        0,
        [
            f"{ROOT} defines no absent profile, building release instead",
            f"{SELECTION} --bin prose --message-format json-render-diagnostics "
            "--profile release"
        ]
    )


def test_corpus_run_stops_where_the_build_reports_no_executable(
    stand_in: Callable[..., Path]
):
    """
    Pins that `corpus_run` exits with the status `jq` gives a filter that
    produced nothing, running no binary, where the build reports none for
    the binary it names.
    """
    assert sourced("corpus_run mutate corpus", stand_in, None).returncode == 4


def test_probe_builds_the_harnesses_over_the_workspace_without_wasm(
    workspace_task: Callable[..., list[str]]
):
    """
    Pins that `rust:probe` builds the probe and corpus binaries, then
    the corpus, imports, and settle harnesses over the workspace without
    `prose_wasm`.
    """
    assert workspace_task("probe") == [
        f"cargo {SELECTION} --bins --profile probe",
        "cargo test --exclude prose_wasm --locked --profile probe --workspace "
        "--no-run --test corpus --test imports --test settle"
    ]


@mark.parametrize(
    ("call", "flags"),
    [
        param(
            "built_binary",
            "--bin prose --message-format json-render-diagnostics --profile probe",
            id = "probe binary"
        ),
        param("corpus_build", "--bins --profile probe --quiet", id="corpus binaries"),
        param(
            "corpus_run mutate corpus",
            "--bin mutate --message-format json-render-diagnostics --profile probe "
            "--quiet",
            id = "corpus binary run"
        )
    ]
)
def test_rows_build_over_the_probe_selection(
    call     : str,
    flags    : str,
    stand_in : Callable[..., Path]
):
    """
    Pins that each binary the delta, mutation, and bake rows build or run
    comes from a build over the packages `rust:probe` builds its binaries
    over, so a full cache hit compiles no dependency again.
    """
    finished = sourced(call, stand_in)

    assert (finished.returncode, finished.stderr.splitlines()) == (
        0, [f"{SELECTION} {flags}"]
    )


def test_sweeps_test_over_the_workspace_without_wasm(
    workspace_task: Callable[..., list[str]]
):
    """
    Pins that `sweep_test`, as `rust:subsets` calls it, runs `cargo test`
    over the workspace without `prose_wasm` under the probe profile, ahead
    of the arguments it is given.
    """
    assert workspace_task("subsets") == [
        "cargo test --exclude prose_wasm --locked --profile probe --workspace "
        "--test settle"
    ]
