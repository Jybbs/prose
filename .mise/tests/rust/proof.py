"""
Pins that `rust:proof` passes `cargo test` every binary `PROOF_TARGETS`
names when it is given no argument, and only the binaries it is given
otherwise, under the profile `PROSE_PROOF_PROFILE` names.
"""

from collections.abc import Callable
from pathlib         import Path
from pytest          import MonkeyPatch, fixture, mark, param
from subprocess      import run

ROOT = Path(__file__).resolve().parents[3]


@fixture(autouse=True)
def environment(monkeypatch: MonkeyPatch, stand_in: Callable[..., Path]):
    """
    Unsets `PROSE_PROOF_PROFILE`, so each case names the profile it runs
    under, and puts a stand-in `cargo` printing its arguments first on
    `PATH`.
    """
    monkeypatch.delenv("PROSE_PROOF_PROFILE", raising=False)
    monkeypatch.setenv("MISE_PROJECT_ROOT", str(ROOT))
    stand_in("cargo")


def proof_arguments(binaries: list[str]) -> list[str]:
    """
    Runs `rust:proof` on `binaries` and returns the arguments the stand-in
    `cargo` received, one per line.
    """
    finished = run(
        [ROOT / ".mise/tasks/rust/proof", *binaries],
        capture_output = True,
        check          = True,
        text           = True
    )
    return finished.stdout.splitlines()


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
    binaries : list[str],
    tests    : list[str]
):
    """
    Pins that one `cargo test` runs over the `prose` package with a
    `--test` flag for each binary the task is given, or for each binary
    `PROOF_TARGETS` names when it is given none, passing no profile.
    """
    assert proof_arguments(binaries) == [
        "test", "--locked", "--package", "prose",
        *tests
    ]


@mark.parametrize(
    ("profile", "flags"),
    [
        param("probe", ["--profile", "probe"], id = "named"),
        param("", [], id = "empty")
    ]
)
def test_proof_builds_under_the_profile_the_environment_names(
    flags       : list[str],
    monkeypatch : MonkeyPatch,
    profile     : str
):
    """
    Pins that `PROSE_PROOF_PROFILE` hands `cargo test` the profile it names,
    and that an empty value passes no profile.
    """
    monkeypatch.setenv("PROSE_PROOF_PROFILE", profile)

    assert proof_arguments(["settle"]) == [
        "test", "--locked", "--package", "prose",
        *flags,
        "--test=settle"
    ]
