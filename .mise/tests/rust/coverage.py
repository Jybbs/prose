"""
Pins the `cargo llvm-cov` runs of `rust:coverage`.
"""

from collections.abc import Callable


def test_coverage_merges_both_halves_of_the_workspace_into_one_report(
    workspace_task: Callable[..., list[str]]
):
    """
    Pins that the task clears the earlier profiles, measures the workspace
    without `prose_wasm` and then `prose_wasm`, and writes one LCOV report,
    running no `cargo test`.
    """
    assert workspace_task("coverage") == [
        "cargo llvm-cov clean --workspace",
        "cargo llvm-cov --no-report --exclude prose_wasm --locked --workspace",
        "cargo llvm-cov --no-report --locked --package prose_wasm",
        "cargo llvm-cov report --lcov --output-path target/lcov.info"
    ]
