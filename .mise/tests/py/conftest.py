"""
Defines the `ran` fixture, which runs a `py` task in a scratch repository
whose `prose` is a stand-in printing each argument it receives on a line of
its own.
"""

from collections.abc import Callable
from pathlib         import Path
from pytest          import MonkeyPatch, fixture
from subprocess      import CompletedProcess, run

ROOT = Path(__file__).resolve().parents[3]


@fixture
def ran(
    monkeypatch : MonkeyPatch,
    scratch     : Callable[[Path], Path],
    stand_in    : Callable[..., Path],
    tmp_path    : Path
) -> Callable[..., CompletedProcess]:
    """
    Builds a function running the `py` task `name` with `args` in a scratch
    repository where git tracks each path in `tracked`, `deleted` naming the
    ones removed from the working tree afterward, under a `GITHUB_ACTIONS`
    of `actions` or none.
    """
    stand_in("prose")
    monkeypatch.delenv("GITHUB_ACTIONS", raising=False)
    monkeypatch.setenv("MISE_PROJECT_ROOT", str(ROOT))

    def run_task(
        name    : str,
        tracked : list[str],
        *args   : str,
        actions : str | None      = None,
        deleted : tuple[str, ...] = ()
    ) -> CompletedProcess:
        """
        Runs the task to completion, capturing its output as text.
        """
        repo = tmp_path / "repo"
        if actions:
            monkeypatch.setenv("GITHUB_ACTIONS", actions)
        for file in tracked:
            (path := repo / file).parent.mkdir(exist_ok=True, parents=True)
            path.write_text("x = 1\n", encoding="utf-8")
        scratch(repo)
        for file in deleted:
            (repo / file).unlink()

        return run(
            [ROOT / ".mise/tasks/py" / name, *args],
            capture_output = True,
            cwd            = repo,
            text           = True
        )

    return run_task
