"""
Defines the `ran` fixture, which runs a `py` task in a scratch repository
whose `prose` is a stand-in printing each argument it receives on a line of
its own.
"""

from collections.abc import Callable
from os              import environ
from pathlib         import Path
from pytest          import fixture
from subprocess      import CompletedProcess, run

ROOT = Path(__file__).resolve().parents[3]


@fixture
def ran(tmp_path: Path) -> Callable[..., CompletedProcess]:
    """
    Returns a function running the `py` task `name` with `args` in a scratch
    repository where git tracks each path in `tracked`, `deleted` naming the
    ones removed from the working tree afterward, under a `GITHUB_ACTIONS`
    of `actions` or none. No `GIT_` variable of the session running the
    suite reaches git, so its index stays out of the scratch repository.
    """
    stand_in = tmp_path / "bin/prose"
    stand_in.parent.mkdir()
    stand_in.write_text("#!/bin/sh\nprintf '%s\\n' \"$@\"\n", encoding="utf-8")
    stand_in.chmod(0o755)

    def run_task(
        name    : str,
        tracked : list[str],
        *args   : str,
        actions : str | None      = None,
        deleted : tuple[str, ...] = ()
    ) -> CompletedProcess:
        """
        Returns the finished run of the task, its output captured as text.
        """
        repo = tmp_path / "repo"
        env  = {
            **{
                key: value for key, value in environ.items()
                if not key.startswith(("GIT_", "GITHUB_ACTIONS"))
            },
            "MISE_PROJECT_ROOT" : str(ROOT),
            "PATH"              : f"{stand_in.parent}:{environ['PATH']}",
            **({"GITHUB_ACTIONS": actions} if actions else {})
        }
        for file in tracked:
            (path := repo / file).parent.mkdir(exist_ok=True, parents=True)
            path.write_text("x = 1\n", encoding="utf-8")
        run(["git", "init", "-q"], check=True, cwd=repo, env=env)
        run(["git", "add", "."], check=True, cwd=repo, env=env)
        for file in deleted:
            (repo / file).unlink()

        return run(
            [ROOT / ".mise/tasks/py" / name, *args],
            capture_output = True,
            cwd            = repo,
            env            = env,
            text           = True
        )

    return run_task
