"""
Pins the `cargo test` run of `rust:suite` and the arguments it hands it.
"""

from collections.abc import Callable
from pytest          import mark, param
from shlex           import join

RUN = (
    "cargo test --bins --exclude prose_wasm --lib --locked --workspace"
    " --test=fixtures --test=imports"
)


@mark.parametrize(
    "args",
    [
        param((), id = "no arguments"),
        param(("reflow_parentheses",), id = "filter"),
        param(("name with space",), id = "argument holding a space"),
        param(("--", "--exact", "name"), id = "runner arguments")
    ]
)
def test_suite_hands_its_arguments_to_cargo_test(
    args           : tuple[str, ...],
    workspace_task : Callable[..., list[str]]
):
    """
    Pins that one `cargo test` runs every test target outside
    `PROOF_TARGETS` with the task's arguments after its own flags, each
    argument holding a space reaching `cargo` whole, and that a run given
    none passes only those flags.
    """
    assert workspace_task("suite", *args) == [join([*RUN.split(), *args])]
