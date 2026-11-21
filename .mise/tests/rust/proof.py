"""
Pins that `rust:proof` passes `cargo test` every binary `PROOF_TARGETS`
names when it is given no argument, and only the binaries it is given
otherwise, under the profile `PROSE_PROOF_PROFILE` names.
"""

from collections.abc import Callable
from pytest          import MonkeyPatch, fixture, mark, param

RUN = "cargo test --locked --package prose"


@fixture(autouse=True)
def unprofiled(monkeypatch: MonkeyPatch):
    """
    Unsets `PROSE_PROOF_PROFILE`, so each case names the profile it runs
    under.
    """
    monkeypatch.delenv("PROSE_PROOF_PROFILE", raising=False)


@mark.parametrize(
    ("binaries", "tests"),
    [
        param([], ["--test=corpus", "--test=settle"], id = "every-binary"),
        param(
            ["corpus", "settle"],
            ["--test=corpus", "--test=settle"],
            id = "both-named"
        ),
        param(["corpus"], ["--test=corpus"], id = "corpus-alone"),
        param(["settle"], ["--test=settle"], id = "settle-alone")
    ]
)
def test_proof_runs_the_binaries_it_is_given(
    binaries       : list[str],
    tests          : list[str],
    workspace_task : Callable[..., list[str]]
):
    """
    Pins that one `cargo test` runs over the `prose` package with a
    `--test` flag for each binary the task is given, or for each binary
    `PROOF_TARGETS` names when it is given none, passing no profile.
    """
    assert workspace_task("proof", *binaries) == [" ".join((RUN, *tests))]


@mark.parametrize(
    ("profile", "flags"),
    [
        param("probe", ["--profile", "probe"], id = "named"),
        param("", [], id = "empty")
    ]
)
def test_proof_builds_under_the_profile_the_environment_names(
    flags          : list[str],
    monkeypatch    : MonkeyPatch,
    profile        : str,
    workspace_task : Callable[..., list[str]]
):
    """
    Pins that `PROSE_PROOF_PROFILE` hands `cargo test` the profile it names,
    and that an empty value passes no profile.
    """
    monkeypatch.setenv("PROSE_PROOF_PROFILE", profile)

    assert workspace_task("proof", "settle") == [
        " ".join((RUN, *flags, "--test=settle"))
    ]
