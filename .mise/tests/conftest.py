"""
Defines the `task` fixture, which loads a task script under
`.mise/tasks/` as a module so a case calls its functions in process, where
pytest-subprocess's `fp` fakes every command the script runs, and the
`workdir` fixture, which poses the files a script reads.
"""

from collections.abc import Callable
from importlib.util  import module_from_spec, spec_from_file_location
from pathlib         import Path
from pytest          import MonkeyPatch, fixture
from types           import ModuleType

TASKS = Path(__file__).resolve().parents[1] / "tasks"


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
