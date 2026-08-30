"""
Running one module of a tree in a fresh interpreter, meaning what each run
left behind, whether a break holds on a second run, and how two runs differ.
"""

from ast                import literal_eval
from concurrent.futures import ThreadPoolExecutor
from functools          import partial
from os                 import close, cpu_count, environ, killpg
from pathlib            import Path
from signal             import SIGKILL
from subprocess         import DEVNULL, PIPE, Popen, TimeoutExpired
from tempfile           import mkstemp

from binary  import last_line
from records import Break, Outcome
from stage   import Stage

MISSING = "no plain constant"
RUNNER  = Path(__file__).with_name("execute.py")


class Runner:
    """
    The interpreter running each module, the pool the runs share, and what
    the original tree left for each module already run from it.
    """

    def __init__(self, python: str, stage: Stage):
        self.python  = python
        self.stage   = stage
        self.timeout = float(environ.get("PROSE_IMPORTS_TIMEOUT", "30"))
        self.pool    = ThreadPoolExecutor(cpu_count())
        self.known   = {}

    def confirm(self, formatted: Path, brk: Break) -> bool:
        """
        Report whether a second run of both sides still breaks `brk` and the
        original agrees with its own first run.
        """
        before = self.execute([self.stage.original], brk.module)
        after  = self.execute([formatted], brk.module)
        steady = before.kind == "ok" and divergence(brk.original, before) is None
        return steady and divergence(before, after) is not None

    def execute(self, trees: list[Path], relative: str) -> Outcome:
        """
        Run the module at `relative` from the first of `trees` carrying it
        in a fresh interpreter and return what it left behind.
        """
        descriptor, record = mkstemp(dir=self.stage.records)
        close(descriptor)
        command = [
            self.python, "-I", "-B",
            str(RUNNER),
            record, relative,
            *map(str, trees)
        ]
        env = {
            "HOME"   : str(self.stage.home),
            "PATH"   : environ["PATH"],
            "TMPDIR" : str(self.stage.tmp)
        }
        child = Popen(
            command,
            cwd               = self.stage.tmp,
            encoding          = "utf-8",
            env               = env,
            errors            = "replace",
            start_new_session = True,
            stderr            = PIPE,
            stdin             = DEVNULL,
            stdout            = DEVNULL
        )
        try:
            _, err = child.communicate(timeout=self.timeout)
        except TimeoutExpired:
            killpg(child.pid, SIGKILL)
            child.stderr.close()
            child.wait()
            return Outcome("timeout", f"times out after {self.timeout:g}s")
        if left := Path(record).read_text(encoding="utf-8"):
            try:
                return Outcome(**literal_eval(left))
            except (SyntaxError, TypeError, ValueError):
                return Outcome("unmeasured", "leaves an unreadable record")
        if child.returncode < 0:
            return Outcome("raised", f"dies on signal {-child.returncode}")
        if child.returncode:
            printed = last_line(err)
            return Outcome("raised", f"exits {child.returncode} printing {printed}")
        return Outcome("unmeasured", "leaves no record")

    def originals(self, modules: list[str]) -> dict[str, Outcome]:
        """
        Return what the original tree leaves for each of `modules`, running
        the ones not yet run.
        """
        fresh = [module for module in modules if module not in self.known]
        self.known.update(self.outcomes(self.stage.original, fresh))
        return {module: self.known[module] for module in modules}

    def outcomes(self, tree: Path, modules: list[str]) -> dict[str, Outcome]:
        """
        Return what each of `modules` leaves behind when run from `tree`,
        the runs sharing the worker pool.
        """
        return dict(zip(modules, self.pool.map(partial(self.execute, [tree]), modules)))


def divergence(original: Outcome, formatted: Outcome) -> tuple[str, str | None] | None:
    """
    Return why `formatted` counts as broken beside `original` and the name
    it hinges on, or `None` where both bind the same namespace.
    """
    if formatted.kind != "ok":
        return formatted.error, formatted.name
    bound, rebound = set(original.names), set(formatted.names)
    if unbound := bound - rebound:
        name = min(unbound)
        return f"leaves `{name}` unbound", name
    if extra := rebound - bound:
        name = min(extra)
        return f"binds `{name}` the original does not", name
    before, after = dict(original.constants), dict(formatted.constants)
    if differing := {name for name, _ in before.items() ^ after.items()}:
        name     = min(differing)
        was, now = before.get(name, MISSING), after.get(name, MISSING)
        return f"binds `{name}` to {now} where the original binds {was}", name
    return None
