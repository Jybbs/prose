"""
Pins the `cargo test` runs of `rust:test` and the arguments it hands them.
"""

from collections.abc import Callable
from pytest          import mark, param
from shlex           import join

RUNS = [
    "cargo test --exclude prose_wasm --locked --workspace",
    "cargo test --locked --package prose_wasm"
]


@mark.parametrize(
    "args",
    [
        param((), id = "no arguments"),
        param(("reflow_parentheses",), id = "filter"),
        param(("name with space",), id = "argument holding a space"),
        param(
            ("--test", "fixtures", "--", "--exact", "name"),
            id = "target and runner arguments"
        )
    ]
)
def test_test_hands_its_arguments_to_each_cargo_test(
    args           : tuple[str, ...],
    workspace_task : Callable[..., list[str]]
):
    """
    Pins that the workspace run without `prose_wasm` and the `prose_wasm`
    run each take the task's arguments after their own flags, each argument
    holding a space reaching `cargo` whole, and that a run given none passes
    only those flags.
    """
    commands = [join([*run.split(), *args]) for run in RUNS]

    assert workspace_task("test", *args) == commands
