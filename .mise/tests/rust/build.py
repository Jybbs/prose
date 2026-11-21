"""
Pins the `cargo build` runs of `rust:build`.
"""

from collections.abc import Callable


def test_build_builds_every_target_of_each_half_of_the_workspace(
    workspace_task: Callable[..., list[str]]
):
    """
    Pins that the task builds every target of the workspace without
    `prose_wasm`, then of `prose_wasm`, and runs no `cargo test`.
    """
    assert workspace_task("build") == [
        "cargo build --all-targets --exclude prose_wasm --locked --workspace",
        "cargo build --all-targets --locked --package prose_wasm"
    ]
