"""
Pins that `rust:probe` and the `sweep_test` the corpus sweeps run each build
over the workspace without `prose_wasm`, the package selection `rust:proof`
builds over.
"""

from collections.abc import Callable


def test_probe_builds_the_harnesses_over_the_workspace_without_wasm(
    workspace_task: Callable[..., list[str]]
):
    """
    Pins that `rust:probe` builds the probe and corpus binaries, then
    the corpus, imports, and settle harnesses over the workspace without
    `prose_wasm`.
    """
    assert workspace_task("probe") == [
        "cargo build --bins --locked -p prose -p prose_corpus --profile probe",
        "cargo test --exclude prose_wasm --locked --no-run --profile probe "
        "--test corpus --test imports --test settle --workspace"
    ]


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
