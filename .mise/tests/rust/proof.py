"""
Pins that `rust:proof` passes `cargo test` every binary `PROOF_TARGETS`
names when it is given no argument, and only the binaries it is given
otherwise, under the profile `PROSE_PROOF_PROFILE` names.
"""

from os         import environ
from pathlib    import Path
from pytest     import MonkeyPatch, fixture, mark, param
from subprocess import run

ROOT     = Path(__file__).resolve().parents[3]
STAND_IN = "#!/bin/sh\nprintf '%s\\n' \"$@\"\n"


@fixture(autouse=True)
def unprofiled(monkeypatch: MonkeyPatch):
    """
    Unsets `PROSE_PROOF_PROFILE`, so each case names the profile it runs
    under.
    """
    monkeypatch.delenv("PROSE_PROOF_PROFILE", raising=False)


def proof_arguments(binaries: list[str], tmp_path: Path) -> list[str]:
    """
    Runs `rust:proof` on `binaries` against a stand-in `cargo` that prints
    its arguments, and returns the arguments one per line.
    """
    (cargo := tmp_path / "cargo").write_text(STAND_IN, encoding="utf-8")
    cargo.chmod(0o755)
    finished = run(
        [ROOT / ".mise/tasks/rust/proof", *binaries],
        capture_output = True,
        check          = True,
        env            = {
            **environ,
            "MISE_PROJECT_ROOT" : str(ROOT),
            "PATH"              : f"{tmp_path}:{environ['PATH']}"
        },
        text = True
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
    tests    : list[str],
    tmp_path : Path
):
    """
    Pins that one `cargo test` runs over the workspace without `prose_wasm`
    with a `--test` flag for each binary the task is given, or for each
    binary `PROOF_TARGETS` names when it is given none, passing no profile.
    """
    assert proof_arguments(binaries, tmp_path) == [
        "test", "--exclude", "prose_wasm", "--locked", "--workspace",
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
    profile     : str,
    tmp_path    : Path
):
    """
    Pins that `PROSE_PROOF_PROFILE` hands `cargo test` the profile it names,
    and that an empty value passes no profile.
    """
    monkeypatch.setenv("PROSE_PROOF_PROFILE", profile)

    assert proof_arguments(["settle"], tmp_path) == [
        "test", "--exclude", "prose_wasm", "--locked", "--workspace",
        *flags,
        "--test=settle"
    ]
