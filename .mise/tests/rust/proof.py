"""
Pins that `rust:proof` passes `cargo test` every binary `PROOF_TARGETS`
names when it is given no argument, and only the binaries it is given
otherwise.
"""

from os         import environ
from pathlib    import Path
from pytest     import mark, param
from subprocess import run

ROOT     = Path(__file__).resolve().parents[3]
STAND_IN = "#!/bin/sh\nprintf '%s\\n' \"$@\"\n"


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
    Pins that one `cargo test` runs over the `prose` package with a
    `--test` flag for each binary the task is given, or for each binary
    `PROOF_TARGETS` names when it is given none.
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

    assert finished.stdout.splitlines() == [
        "test", "--locked", "--package", "prose",
        *tests
    ]
