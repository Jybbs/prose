"""
Pins which changes since `origin/main` lead `rust:affected` to run the Rust
suite, and the arguments it hands `cargo test`.
"""

from collections.abc import Callable
from pytest          import mark, param
from subprocess      import CompletedProcess

SUITE = [
    "cargo test --bins --exclude prose_wasm --lib --locked --workspace",
    "cargo test --locked --package prose_wasm"
]


@mark.parametrize(
    "changed",
    [
        param(".cargo/clippy.toml", id = "clippy config"),
        param(".cargo/config.toml", id = "cargo config"),
        param(".mise/config.toml", id = "tool pins"),
        param("Cargo.lock", id = "workspace lockfile"),
        param("Cargo.toml", id = "workspace manifest"),
        param("corpus/src/lib.rs", id = "corpus"),
        param("crate/src/lib.rs", id = "crate"),
        param("crate/tests/fixtures/case/input.py", id = "fixture"),
        param("harness/src/lib.rs", id = "harness"),
        param("tools/bench/src/lib.rs", id = "nested member"),
        param("wasm/src/lib.rs", id = "wasm")
    ]
)
def test_affected_runs_the_suite_once_a_change_reaches_a_rust_path(
    affected : Callable[..., CompletedProcess],
    changed  : str
):
    """
    Pins that a change to a Rust source, a workspace member `cargo metadata`
    lists, or a file configuring the workspace's build or its toolchain runs
    both halves of `rust:suite`.
    """
    finished = affected(changed, "edited", group="rust")

    assert finished.returncode == 0
    assert finished.stdout.splitlines() == SUITE


@mark.parametrize(
    "changed",
    [
        param(".cargo/deny.toml", id = "dependency policy"),
        param(".mise/tasks/py/format", id = "task"),
        param("crates/src/lib.rs", id = "prefix"),
        param("site/index.md", id = "site"),
        param("src/Cargo.toml", id = "nested manifest")
    ]
)
def test_affected_skips_the_suite_where_no_change_reaches_a_rust_path(
    affected : Callable[..., CompletedProcess],
    changed  : str
):
    """
    Pins that a change outside every Rust path, the `cargo-deny` policy
    beside the build configuration, a folder sharing a Rust folder's prefix,
    and a manifest outside the workspace root included, runs no `cargo` and
    exits zero on a line saying so.
    """
    finished = affected(changed, "edited", group="rust")

    assert finished.returncode == 0
    assert finished.stdout == "No Rust file changed since origin/main\n"


def test_affected_fails_where_the_repository_has_no_origin_main(
    affected: Callable[..., CompletedProcess]
):
    """
    Pins that a repository with no `origin/main` to compare against fails
    the task before it runs or skips the suite.
    """
    finished = affected("crate/src/lib.rs", "edited", group="rust", main=False)

    assert finished.returncode != 0
    assert finished.stdout == ""


@mark.parametrize("how", ["committed", "untracked"])
def test_affected_reads_committed_and_untracked_changes(
    affected : Callable[..., CompletedProcess],
    how      : str
):
    """
    Pins that a committed change and a file git does not yet track reach the
    task, as the edit in place every other case makes does.
    """
    assert affected("crate/src/lib.rs", how, group="rust").stdout.splitlines() == SUITE


@mark.parametrize(
    ("changed", "how", "upstream"),
    [
        param("crate/target/debug/prose", "ignored", (), id = "ignored file"),
        param(
            "site/index.md",
            "edited",
            ("crate/src/lib.rs",),
            id = "change on origin/main"
        )
    ]
)
def test_affected_skips_the_suite_for_changes_the_branch_did_not_make(
    affected : Callable[..., CompletedProcess],
    changed  : str,
    how      : str,
    upstream : tuple[str, ...]
):
    """
    Pins that a file git ignores and a change `origin/main` gained after the
    branch point reach no Rust path.
    """
    finished = affected(changed, how, group="rust", upstream=upstream)

    assert finished.stdout == "No Rust file changed since origin/main\n"


def test_affected_hands_its_arguments_to_each_cargo_test(
    affected: Callable[..., CompletedProcess]
):
    """
    Pins that the arguments the task is given follow the flags of both
    `cargo test` runs.
    """
    finished = affected(
        "crate/src/lib.rs",
        "edited",
        "--",
        "--exact",
        "name",
        group = "rust"
    )

    assert finished.stdout.splitlines() == [f"{line} -- --exact name" for line in SUITE]
