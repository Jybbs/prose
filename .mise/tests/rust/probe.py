"""
Pins that `rust:probe` builds the sweep harnesses over the package
selection `rust:proof` and the corpus sweeps build them over, so the cache
it warms holds the feature set each of them resolves.
"""

from os         import environ
from pathlib    import Path
from subprocess import run

ROOT     = Path(__file__).resolve().parents[3]
STAND_IN = "#!/bin/sh\nprintf '%s\\n' \"$*\"\n"


def cargo_calls(command: list[Path | str], tmp_path: Path) -> list[str]:
    """
    Runs `command` against a stand-in `cargo` that prints its arguments,
    and returns each call's arguments as one line.
    """
    (cargo := tmp_path / "cargo").write_text(STAND_IN, encoding="utf-8")
    cargo.chmod(0o755)
    finished = run(
        command,
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


def test_probe_builds_the_harnesses_over_the_workspace_less_wasm(
    tmp_path: Path
):
    """
    Pins that `rust:probe` builds the probe and corpus binaries, then the
    corpus, imports, and settle harnesses over the workspace less
    `prose_wasm`.
    """
    assert cargo_calls([ROOT / ".mise/tasks/rust/probe"], tmp_path) == [
        "build --bins --locked -p prose -p prose_corpus --profile probe",
        "test --exclude prose_wasm --locked --no-run --profile probe "
        "--test corpus --test imports --test settle --workspace"
    ]


def test_sweeps_test_over_the_workspace_less_wasm(tmp_path: Path):
    """
    Pins that `sweep_test` runs `cargo test` over the workspace less
    `prose_wasm` under the probe profile, ahead of the arguments it is
    given.
    """
    script = f'source "{ROOT}/.mise/lib/corpus.bash"; sweep_test --test settle'

    assert cargo_calls(["bash", "-euo", "pipefail", "-c", script], tmp_path) == [
        "test --exclude prose_wasm --locked --profile probe --workspace "
        "--test settle"
    ]
