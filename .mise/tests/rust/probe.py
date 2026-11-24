"""
Pins that `rust:probe` and the `sweep_test` the corpus sweeps run each build
over the workspace without `prose_wasm`, the package selection `rust:proof`
builds over, and that the delta, mutation, and bake rows build their binaries
over the packages `rust:probe` builds its binaries over.
"""

from os         import environ
from pathlib    import Path
from pytest     import mark, param
from subprocess import run

ROOT      = Path(__file__).resolve().parents[3]
SELECTION = "build --locked -p prose -p prose_corpus"
STAND_IN  = (
    "#!/bin/sh\n"
    "printf '%s\\n' \"$*\" >&2\n"
    "printf '{\"executable\":\"true\",\"target\":{\"name\":\"%s\"}}\\n' \"$8\"\n"
)


def cargo_calls(command: list[Path | str], tmp_path: Path) -> list[str]:
    """
    Runs `command` from the checkout against a stand-in `cargo` that prints
    its arguments to stderr and reports the binary each call names as
    `true`, and returns each call's arguments as one line.
    """
    (cargo := tmp_path / "cargo").write_text(STAND_IN, encoding="utf-8")
    cargo.chmod(0o755)
    finished = run(
        command,
        capture_output = True,
        check          = True,
        cwd            = ROOT,
        env            = {
            **environ,
            "MISE_PROJECT_ROOT" : str(ROOT),
            "PATH"              : f"{tmp_path}:{environ['PATH']}"
        },
        text = True
    )
    return finished.stderr.splitlines()


def test_probe_builds_the_harnesses_over_the_workspace_without_wasm(tmp_path: Path):
    """
    Pins that `rust:probe` builds the probe and corpus binaries, then
    the corpus, imports, and settle harnesses over the workspace without
    `prose_wasm`.
    """
    assert cargo_calls([ROOT / ".mise/tasks/rust/probe"], tmp_path) == [
        f"{SELECTION} --bins --profile probe",
        "test --exclude prose_wasm --locked --profile probe --workspace "
        "--no-run --test corpus --test imports --test settle"
    ]


@mark.parametrize(
    ("call", "flags"),
    [
        param(
            "built_binary",
            "--bin prose --message-format json-render-diagnostics --profile probe",
            id = "probe binary"
        ),
        param("corpus_build", "--bins --profile probe --quiet", id="corpus binaries"),
        param(
            "corpus_run mutate corpus",
            "--bin mutate --message-format json-render-diagnostics --profile probe "
            "--quiet",
            id = "corpus binary run"
        )
    ]
)
def test_rows_build_over_the_probe_selection(call: str, flags: str, tmp_path: Path):
    """
    Pins that each binary the delta, mutation, and bake rows build or run
    comes from a build over the packages `rust:probe` builds its binaries
    over, so a full cache hit compiles no dependency again.
    """
    script = (
        f'source "{ROOT}/.mise/lib/corpus.bash"; '
        f'source "{ROOT}/.mise/lib/delta.bash"; {call}'
    )

    assert cargo_calls(["bash", "-euo", "pipefail", "-c", script], tmp_path) == [
        f"{SELECTION} {flags}"
    ]


def test_sweeps_test_over_the_workspace_without_wasm(tmp_path: Path):
    """
    Pins that `sweep_test` runs `cargo test` over the workspace without
    `prose_wasm` under the probe profile, ahead of the arguments it is
    given.
    """
    script = f'source "{ROOT}/.mise/lib/corpus.bash"; sweep_test --test settle'

    assert cargo_calls(["bash", "-euo", "pipefail", "-c", script], tmp_path) == [
        "test --exclude prose_wasm --locked --profile probe --workspace "
        "--test settle"
    ]
