"""
Defines the `task` fixture, which loads a task script under
`.mise/tasks/` as a module so a case calls its functions in process, where
pytest-subprocess's `fp` fakes every command the script runs, the `workdir`
fixture, which writes the files a script reads into a scratch tree, and the
`affected` fixture, which runs an `affected` task in a scratch repository
changed since its `origin/main`.
"""

from collections.abc import Callable, Iterable
from functools       import partial
from importlib.util  import module_from_spec, spec_from_file_location
from json            import dumps
from os              import environ
from pathlib         import Path
from pytest          import MonkeyPatch, fixture
from shlex           import quote
from subprocess      import CompletedProcess, run
from types           import ModuleType

MEMBERS = ("corpus", "crate", "harness", "tools/bench", "wasm")
TASKS   = Path(__file__).resolve().parents[1] / "tasks"


@fixture
def affected(tmp_path: Path) -> Callable[..., CompletedProcess]:
    """
    Builds stand-in `cargo` and `uv` commands, each printing the command
    it receives on a line of its own, with `cargo metadata` printing a
    workspace of `MEMBERS` with no test target, and an environment where git
    reads neither the configuration nor the `GIT_` variables of the session
    running the suite.

    Returns:
        A function running the `affected` task of `group` with `args` in a
        scratch repository whose `origin/main` tracks each path in
        `tracked`, after `changed` is changed in the way `how` names.
    """
    metadata = dumps(
        {
            "packages"       : [
                {"manifest_path": f"/workspace/{member}/Cargo.toml", "targets": []}
                for member in MEMBERS
            ],
            "workspace_root" : "/workspace"
        }
    )
    stand_in = tmp_path / "bin/cargo"
    stand_in.parent.mkdir()
    stand_in.write_text(
        "#!/bin/sh\n"
        f'[ "$1" = metadata ] && exec echo {quote(metadata)}\n'
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

    def write(files: Iterable[str], text: str):
        """
        Writes `text` to each path in `files` under the scratch repository,
        making every parent directory the path names.
        """
        for file in files:
            (path := repo / file).parent.mkdir(exist_ok=True, parents=True)
            path.write_text(text, encoding="utf-8")

    def run_task(
        changed   : str,
        how       : str,
        *args     : str,
        group     : str,
        alongside : tuple[str, ...] = (),
        main      : bool            = True,
        reading   : tuple[str, ...] = (),
        tracked   : tuple[str, ...] = (),
        upstream  : tuple[str, ...] = ()
    ) -> CompletedProcess:
        """
        Commits the scratch repository, each path in `reading` naming
        `changed` in its text, adds each path in `upstream` on `origin/main`
        after the branch point, changes `changed` and each path in
        `alongside` in the way `how` names, and runs the task, leaving out
        `origin/main` where `main` is `False`.

        Args:
            how: One of `committed`, `deleted`, `edited`, `ignored`,
                 `renamed`, or `untracked`.

        Returns:
            The finished run, its output captured as text.
        """
        changes = {changed, *alongside}
        write({"seed", *changes, *tracked}, "x = 1\n")
        write(reading, f'ROOT / "{changed}"\n')
        git(["git", "init", "-q"])
        if how == "ignored":
            (repo / ".git/info/exclude").write_text(
                "\n".join(changes),
                encoding = "utf-8"
            )
        git(["git", "add", "."])
        if how == "untracked":
            git(["git", "rm", "--cached", "-q", *changes])
        git(["git", "commit", "-qm", "main"])
        if upstream:
            git(["git", "switch", "-q", "-c", "upstream"])
            write(upstream, "x = 1\n")
            git(["git", "add", "."])
            git(["git", "commit", "-qm", "upstream"])
        if main:
            git(["git", "update-ref", "refs/remotes/origin/main", "HEAD"])
        if upstream:
            git(["git", "switch", "-q", "-"])
        for file in changes:
            if how == "deleted":
                (repo / file).unlink()
            elif how == "renamed":
                git(["git", "mv", file, f"{file}-renamed"])
            else:
                (repo / file).write_text("x = 2\n", encoding="utf-8")
        if how in {"committed", "renamed"}:
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
    Builds a loader for the task scripts under `.mise/tasks/`.

    Returns:
        A function loading the task script at `<group>/<name>.py`.
    """
    def load(name: str) -> ModuleType:
        """
        Executes the script `name` as a module, without running its
        `__main__` block.

        Returns:
            The module the script defines.
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
    Builds a writer of scratch trees under the case's temporary directory.

    Returns:
        A function writing each file a mapping names into the scratch tree
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
