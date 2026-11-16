"""
Defines the `task` fixture, which loads a task script under
`.mise/tasks/` as a module so a case calls its functions in process, where
pytest-subprocess's `fp` fakes every command the script runs, the `workdir`
fixture, which writes the files a script reads into a scratch tree, and the
`affected` fixture, which runs an `affected` task in a scratch repository
changed since its `origin/main`.
"""

from collections.abc import Callable
from functools       import partial
from importlib.util  import module_from_spec, spec_from_file_location
from os              import environ
from pathlib         import Path
from pytest          import MonkeyPatch, fixture
from subprocess      import CompletedProcess, run
from types           import ModuleType

TASKS = Path(__file__).resolve().parents[1] / "tasks"


@fixture
def affected(tmp_path: Path) -> Callable[..., CompletedProcess]:
    """
    Returns a function running the `affected` task of `group` with `args` in
    a scratch repository whose `origin/main` tracks each path in `tracked`,
    after `changed` is changed in the way `how` names. `cargo` and `uv`
    are stand-ins printing each command they receive on a line of its own,
    `cargo metadata` printing a workspace with no test target, and git
    reads neither the configuration nor the `GIT_` variables of the session
    running the suite.
    """
    stand_in = tmp_path / "bin/cargo"
    stand_in.parent.mkdir()
    stand_in.write_text(
        "#!/bin/sh\n"
        '[ "$1" = metadata ] && exec echo \'{"packages": []}\'\n'
        'echo "${0##*/} $*"\n',
        encoding = "utf-8"
    )
    stand_in.chmod(0o755)
    (stand_in.parent / "uv").symlink_to(stand_in)
    repo = tmp_path / "repo"
    git  = partial(
        run,
        check = True,
        cwd   = repo,
        env   = {
            **{name: text for name, text in environ.items() if name[:4] != "GIT_"},
            "GIT_AUTHOR_EMAIL"    : "author@example.com",
            "GIT_AUTHOR_NAME"     : "author",
            "GIT_COMMITTER_EMAIL" : "author@example.com",
            "GIT_COMMITTER_NAME"  : "author",
            "GIT_CONFIG_GLOBAL"   : "/dev/null",
            "GIT_CONFIG_NOSYSTEM" : "1",
            "MISE_PROJECT_ROOT"   : str(TASKS.parents[1]),
            "PATH"                : f"{stand_in.parent}:{environ['PATH']}"
        }
    )

    def run_task(
        changed   : str,
        how       : str,
        *args     : str,
        group     : str,
        alongside : tuple[str, ...] = (),
        main      : bool            = True,
        tracked   : tuple[str, ...] = ()
    ) -> CompletedProcess:
        """
        Returns the finished run of the task, its output captured as text,
        where `how` is `committed`, `deleted`, `edited`, or `untracked`
        and applies to each path in `alongside` as well, and `main` set to
        `False` leaves the repository without an `origin/main`.
        """
        changes = {changed, *alongside}
        for file in {"README.md", *changes, *tracked}:
            (path := repo / file).parent.mkdir(exist_ok=True, parents=True)
            path.write_text("x = 1\n", encoding="utf-8")
        git(["git", "init", "-q"])
        git(["git", "add", "."])
        if how == "untracked":
            git(["git", "rm", "--cached", "-q", *changes])
        git(["git", "commit", "-qm", "main"])
        if main:
            git(["git", "update-ref", "refs/remotes/origin/main", "HEAD"])
        for file in changes:
            if how == "deleted":
                (repo / file).unlink()
            else:
                (repo / file).write_text("x = 2\n", encoding="utf-8")
        if how == "committed":
            git(["git", "commit", "-qam", "change"])

        return git(
            [TASKS / group / "affected", *args],
            capture_output = True,
            check          = False,
            text           = True
        )

    return run_task


@fixture(scope="session")
def task() -> Callable[[str], ModuleType]:
    """
    Returns a function loading the task script at `<group>/<name>.py` under
    `.mise/tasks/`.
    """
    def load(name: str) -> ModuleType:
        """
        Returns the module the script `name` defines, without running its
        `__main__` block.
        """
        spec   = spec_from_file_location(name.replace("/", "_"), TASKS / f"{name}.py")
        module = module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    return load


@fixture
def workdir(
    monkeypatch : MonkeyPatch,
    tmp_path    : Path
) -> Callable[[dict[str, str]], Path]:
    """
    Returns a function writing each file a mapping names into a scratch tree
    and making that tree the current directory, which it returns.
    """
    def write(files: dict[str, str]) -> Path:
        """
        Writes the text of each file in `files` at its path under the tree,
        making every parent directory the path names.
        """
        for name, text in files.items():
            (target := tmp_path / name).parent.mkdir(exist_ok=True, parents=True)
            target.write_text(text, encoding="utf-8")
        monkeypatch.chdir(tmp_path)
        return tmp_path

    return write
