"""
Running one module of a tree in a fresh interpreter, meaning what each run
left behind and whether a break holds on a second run.
"""

from ast                import literal_eval
from concurrent.futures import ThreadPoolExecutor
from contextlib         import suppress
from functools          import partial
from os                 import close, environ, killpg, process_cpu_count
from pathlib            import Path
from signal             import SIGKILL, Signals
from subprocess         import DEVNULL, PIPE, Popen, TimeoutExpired
from tempfile           import mkstemp

from binary     import last_line
from comparison import divergence
from records    import Break, Outcome
from stage      import Stage

RUNNER = Path(__file__).with_name("execute.py")


class Runner:
    """
    The interpreter running each module, the pool the runs share, and what
    the original tree left for each module already run from it.
    """

    def __init__(self, python: str, stage: Stage):
        self.python  = python
        self.stage   = stage
        self.timeout = float(environ.get("PROSE_IMPORTS_TIMEOUT", "30"))
        self.pool    = ThreadPoolExecutor(process_cpu_count())
        self.known   = {}

    def confirm(self, brk: Break, formatted: Path) -> bool:
        """
        Report whether the original agrees with its own first run and, where
        it does, whether a second run of the formatted side still breaks
        `brk`.
        """
        before = self.execute(brk.module, [self.stage.original])

        return (
            before.kind == "ok"
            and divergence(before, brk.original) is None
            and divergence(self.execute(brk.module, [formatted]), before) is not None
        )

    def execute(self, relative: str, trees: list[Path]) -> Outcome:
        """
        Run the module at `relative` from the first of `trees` carrying it
        in a fresh interpreter and return what it left behind.
        """
        descriptor, record = mkstemp(dir=self.stage.records)
        close(descriptor)

        with Popen(
            [self.python, "-I", "-B", str(RUNNER), record, relative, *map(str, trees)],
            cwd      = self.stage.tmp,
            encoding = "utf-8",
            env      = {
                "HOME"   : str(self.stage.home),
                "PATH"   : environ["PATH"],
                "TMPDIR" : str(self.stage.tmp)
            },
            errors            = "replace",
            start_new_session = True,
            stderr            = PIPE,
            stdin             = DEVNULL,
            stdout            = DEVNULL
        ) as child:
            try:
                _, err = child.communicate(timeout=self.timeout)
            except TimeoutExpired:
                with suppress(ProcessLookupError):
                    killpg(child.pid, SIGKILL)
                return Outcome("timeout", error=f"times out after {self.timeout:g}s")

        if left := Path(record).read_text(encoding="utf-8"):
            try:
                return Outcome(**literal_eval(left))
            except (SyntaxError, TypeError, ValueError):
                return Outcome("unmeasured", error="leaves an unreadable record")

        if child.returncode < 0:
            return Outcome("raised", error=f"dies on {Signals(-child.returncode).name}")

        if child.returncode:
            return Outcome(
                "raised",
                error = f"exits {child.returncode} printing {last_line(err)}"
            )

        return Outcome("unmeasured", error="leaves no record")

    def originals(self, modules: list[str]) -> dict[str, Outcome]:
        """
        Return what the original tree leaves for each of `modules`, running
        the ones not yet run.
        """
        self.known.update(
            self.outcomes(
                [module for module in modules if module not in self.known],
                self.stage.original
            )
        )

        return {module: self.known[module] for module in modules}

    def outcomes(self, modules: list[str], tree: Path) -> dict[str, Outcome]:
        """
        Return what each of `modules` leaves behind when run from `tree`,
        the runs sharing the worker pool.
        """
        return dict(
            zip(modules, self.pool.map(partial(self.execute, trees=[tree]), modules))
        )

