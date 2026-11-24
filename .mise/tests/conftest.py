"""
Defines the `task` fixture, which loads a task script under
`.mise/tasks/` as a module so a case calls its functions in process, where
pytest-subprocess's `fp` fakes every command the script runs, the `workdir`
fixture, which writes the files a script reads into a scratch tree, the
`scratch` fixture, which makes a directory a git repository isolated from
the session running the suite, the `stand_in` fixture, which writes a
stand-in command first on `PATH`, the `affected` fixture, which runs an
`affected` task in a scratch repository changed since its `origin/main`,
and the `delta` fixture, which runs a delta task in a scratch clone against
a stand-in build. Each session writes under a base temporary directory of
its own.
"""

from collections.abc import Callable, Iterable
from functools       import partial
from importlib.util  import module_from_spec, spec_from_file_location
from json            import dumps
from os              import environ, pathsep
from pathlib         import Path
from pytest          import (
    Config, ExitCode, MonkeyPatch, Session, StashKey, fixture, hookimpl
)
from shlex      import quote
from shutil     import rmtree
from subprocess import CompletedProcess, run
from tempfile   import mkdtemp
from types      import ModuleType

MEMBERS = {
    "corpus"      : {},
    "crate"       : {"harness": "dev", "tools/core": None},
    "harness"     : {"crate": None},
    "tools/bench" : {},
    "tools/core"  : {},
    "wasm"        : {"crate": None}
}
OWNED = StashKey[Path]()
TASKS = Path(__file__).resolve().parents[1] / "tasks"


@hookimpl(tryfirst=True)
def pytest_configure(config: Config):
    """
    Points `--basetemp` at a fresh directory under the system temporary
    folder where the command line names none, outside the `pytest-of-<user>`
    root whose older directories each session prunes as it exits.
    """
    if config.option.basetemp is None:
        config.option.basetemp = config.stash[OWNED] = Path(mkdtemp(prefix="pytest-"))


@hookimpl(trylast=True)
def pytest_sessionfinish(exitstatus: int, session: Session):
    """
    Removes the directory `pytest_configure` made once every case passed,
    keeping it after a failure for the failing cases' files.
    """
    if exitstatus == ExitCode.OK and (root := session.config.stash.get(OWNED, None)):
        rmtree(root, ignore_errors=True)


@fixture
def affected(
    scratch  : Callable[[Path], Path],
    stand_in : Callable[..., Path],
    tmp_path : Path
) -> Callable[..., CompletedProcess]:
    """
    Builds stand-in `cargo`, `uv`, and `vitest` commands, each printing the
    command it receives on a line of its own, with `cargo metadata` printing
    a workspace of `MEMBERS` with no test target, each member depending on
    the members it maps to under the kind each maps to, and puts them first
    on `PATH`.

    Returns:
        A function running the `affected` task of `group` with `args` in a
        scratch repository whose `origin/main` tracks each path in
        `tracked`, after `changed` is changed in the way `how` names.
    """
    metadata = dumps(
        {
            "packages"       : [
                {
                    "dependencies"  : [
                        {"kind": kind, "path": f"/workspace/{needed}"}
                        for needed, kind in needs.items()
                    ],
                    "manifest_path" : f"/workspace/{member}/Cargo.toml",
                    "name"          : f"prose_{Path(member).name}",
                    "targets"       : []
                }
                for member, needs in MEMBERS.items()
            ],
            "workspace_root" : "/workspace"
        }
    )
    cargo = stand_in(
        "cargo",
        f'[ "$1" = metadata ] && exec echo {quote(metadata)}\n'
        'echo "${0##*/} $*"\n'
    )
    for name in ("uv", "vitest"):
        (cargo.parent / name).symlink_to(cargo)
    repo = tmp_path / "repo"
    git  = partial(run, check=True, cwd=repo)

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
        scratch(repo)
        if how in {"ignored", "untracked"}:
            git(["git", "rm", "--cached", "-q", *changes])
        if how == "ignored":
            (repo / ".git/info/exclude").write_text(
                "\n".join(changes),
                encoding = "utf-8"
            )
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


@fixture
def delta(
    monkeypatch : MonkeyPatch,
    scratch     : Callable[[Path], Path],
    stand_in    : Callable[..., Path],
    tmp_path    : Path
) -> Callable[..., CompletedProcess]:
    """
    Clones a scratch repository into `clone`, alone beside its upstream,
    on a `feature` branch whose formatter writes a module of the corpus
    as `head` where `main` writes it as `main`. A stand-in `cargo` builds
    the formatter of the tree it runs in, counts any binary already in
    the target directory as built, and reports the files whose formatting
    differs, beside stand-in `mise` and `python` commands.

    Returns:
        A function running the task at `name` in the clone with `args` under
        the environment variables in `variables`, which returns the finished
        run, its output captured as text.
    """
    clone, corpus, upstream = (
        tmp_path / name for name in ("clone", "corpus", "upstream")
    )
    for name in [name for name in environ if name.startswith(("CARGO_", "PROSE_"))]:
        monkeypatch.delenv(name)
    for name, value in {
        "MISE_PROJECT_ROOT"  : clone,
        "PROSE_DELTA_WIDTHS" : "88",
        "RUNNER_TEMP"        : tmp_path,
        "TMPDIR"             : tmp_path
    }.items():
        monkeypatch.setenv(name, str(value))
    stand_in(
        "cargo",
        'case "$*" in\n'
        '  "build --bin prose --locked --message-format json-render-diagnostics'
        ' --profile probe")\n'
        "    binary=${CARGO_TARGET_DIR:-$PWD/target}/probe/prose\n"
        '    mkdir -p "${binary%/*}"\n'
        '    [ -e "$binary" ] || cp crate/src/formatter "$binary"\n'
        '    chmod +x "$binary"\n'
        '    printf \'{"executable":"%s","target":{"name":"prose"}}\' "$binary"\n'
        "    ;;\n"
        '  "build --bins --locked -p prose_corpus --profile probe --quiet") ;;\n'
        '  "run --bin delta --locked -p prose_corpus --profile probe --quiet -- "*)\n'
        "    shift 10\n"
        '    git -C "$1" diff --name-only "base-$2" "head-$2"\n'
        "    ;;\n"
        '  *) echo "unexpected: $*" >&2; exit 1 ;;\n'
        "esac\n"
    )
    stand_in(
        "mise",
        f'[ "$*" = "run rust:bake" ] && exec {quote(str(TASKS / "rust/bake"))}\n'
    )
    stand_in(
        "python",
        f'case "$2" in *sysconfig*) echo {quote(str(corpus))} ;; *) echo 3.15 ;; esac\n'
    )

    def formatter(root: Path, value: str):
        """
        Writes the formatter `root` builds, which writes each module of the
        corpus as `value`.
        """
        (root / "crate/src/formatter").write_text(
            f'#!/bin/sh\necho "x = \'{value}\'" > "$5/module.py"\n'
            'echo \'{"files_visited":1,"kind":"summary","schema_version":1}\'\n',
            encoding = "utf-8"
        )

    corpus.mkdir()
    (corpus / "module.py").write_text("x = ''\n", encoding="utf-8")
    (upstream / "crate/src").mkdir(parents=True)
    (upstream / "Cargo.toml").write_text("[profile.probe]\n", encoding="utf-8")
    formatter(upstream, "main")
    for root, *command in (
        (scratch(upstream), "commit", "-qm", "main"),
        (upstream, "branch", "-M", "main"),
        (tmp_path, "clone", "-q", upstream.as_uri(), clone),
        (clone, "switch", "-c", "feature", "-q")
    ):
        run(["git", *command], check=True, cwd=root)
    formatter(clone, "head")
    run(["git", "commit", "-qam", "head"], check=True, cwd=clone)
    (clone / ".mise").symlink_to(TASKS.parent)

    def run_task(name: str, *args: str, **variables: str) -> CompletedProcess:
        """
        Runs `.mise/tasks/<name>` in the clone.

        Returns:
            The finished run, its output captured as text.
        """
        return run(
            [TASKS / name, *args],
            capture_output = True,
            cwd            = clone,
            env            = {**environ, **variables},
            text           = True
        )

    return run_task


@fixture
def scratch(monkeypatch: MonkeyPatch) -> Callable[[Path], Path]:
    """
    Builds an environment where git reads neither the configuration nor the
    `GIT_` variables of the session running the suite, and commits under a
    fixed author and committer.

    Returns:
        A function running `git init` in `root` and staging every file
        under it, which returns `root`.
    """
    for name in [name for name in environ if name.startswith("GIT_")]:
        monkeypatch.delenv(name)
    for name, value in {
        "GIT_AUTHOR_EMAIL"    : "author@example.com",
        "GIT_AUTHOR_NAME"     : "author",
        "GIT_COMMITTER_EMAIL" : "author@example.com",
        "GIT_COMMITTER_NAME"  : "author",
        "GIT_CONFIG_GLOBAL"   : "/dev/null",
        "GIT_CONFIG_NOSYSTEM" : "1"
    }.items():
        monkeypatch.setenv(name, value)

    def init(root: Path) -> Path:
        """
        Makes `root` a git repository tracking every file under it.
        """
        for command in (["init", "-q"], ["add", "."]):
            run(["git", *command], check=True, cwd=root)
        return root

    return init


@fixture
def stand_in(monkeypatch: MonkeyPatch, tmp_path: Path) -> Callable[..., Path]:
    """
    Builds a writer of stand-in commands under the case's `bin` directory,
    which it puts first on `PATH` beside a `MISE_PROJECT_ROOT` naming the
    checkout a task runs from.

    Returns:
        A function writing `script` as the executable `name` and
        returning its path, where the default `script` prints each argument
        it receives on a line of its own.
    """
    (folder := tmp_path / "bin").mkdir()
    monkeypatch.setenv("MISE_PROJECT_ROOT", str(TASKS.parents[1]))
    monkeypatch.setenv("PATH", str(folder), prepend=pathsep)

    def write(name: str, script: str = "printf '%s\\n' \"$@\"\n") -> Path:
        """
        Writes `script` under a `sh` shebang as the executable `name`.
        """
        (command := folder / name).write_text(f"#!/bin/sh\n{script}", encoding="utf-8")
        command.chmod(0o755)
        return command

    return write


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
