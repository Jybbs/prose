"""
Defines the `workspace_task` fixture, which runs a task sourcing
`.mise/lib/workspace.bash` against a stand-in `cargo`.
"""

from collections.abc import Callable
from json            import dumps
from os              import environ
from pathlib         import Path
from pytest          import MonkeyPatch, fixture
from subprocess      import run
from sys             import executable

ROOT    = Path(__file__).resolve().parents[3]
TARGETS = ["corpus", "fixtures", "imports", "settle"]


@fixture
def workspace_task(
    monkeypatch : MonkeyPatch,
    tmp_path    : Path
) -> Callable[..., list[str]]:
    """
    Builds a stand-in `cargo` printing each command it receives on a line of
    its own, each argument quoted as a shell reads it, with `cargo metadata`
    printing one package holding a test target for each name in `TARGETS`,
    and puts it first on `PATH`.

    Returns:
        A function running the `rust` task `name` with `args`, which returns
        the commands the task ran.
    """
    metadata = dumps(
        {
            "packages": [
                {"targets": [{"kind": ["test"], "name": name} for name in TARGETS]}
            ]
        }
    )
    (cargo := tmp_path / "cargo").write_text(
        f"#!{executable}\n"
        "from shlex import join\n"
        "from sys   import argv\n"
        "command = join(['cargo', *argv[1:]])\n"
        f"print({metadata!r} if argv[1:2] == ['metadata'] else command)\n",
        encoding = "utf-8"
    )
    cargo.chmod(0o755)
    monkeypatch.setenv("MISE_PROJECT_ROOT", str(ROOT))
    monkeypatch.setenv("PATH", f"{tmp_path}:{environ['PATH']}")

    def run_task(name: str, *args: str) -> list[str]:
        """
        Runs `.mise/tasks/rust/<name>` with `args` in the case's temporary
        directory.

        Returns:
            The lines the task printed.
        """
        finished = run(
            [ROOT / ".mise/tasks/rust" / name, *args],
            capture_output = True,
            check          = True,
            cwd            = tmp_path,
            text           = True
        )
        return finished.stdout.splitlines()

    return run_task
