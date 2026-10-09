"""
Defines the `task` fixture, which loads a task script under
`.mise/tasks/` as a module so a case calls its functions in process, where
pytest-subprocess's `fp` fakes every command the script runs.
"""

from collections.abc     import Callable
from importlib.machinery import SourceFileLoader
from importlib.util      import module_from_spec, spec_from_loader
from pathlib             import Path
from pytest              import fixture
from types               import ModuleType

TASKS = Path(__file__).resolve().parents[1] / "tasks"


@fixture(scope="session")
def task() -> Callable[[str], ModuleType]:
    """
    Returns a function loading the task script at `<group>/<name>` under
    `.mise/tasks/`, which names no `.py` suffix for the import system to
    find.
    """
    def load(name: str) -> ModuleType:
        """
        Returns the module the script `name` defines, without running its
        `__main__` block.
        """
        loader = SourceFileLoader(name.replace("/", "_"), str(TASKS / name))
        module = module_from_spec(spec_from_loader(loader.name, loader))
        loader.exec_module(module)
        return module

    return load
