"""
One sweep of a corpus at a width, meaning a formatted copy, the run of every
module the formatter rewrote from both trees, and each break confirmed and
attributed.
"""

from functools import partial

from attribution import Attributor
from binary      import format_tree
from comparison  import compare
from corpus      import candidates, interpreter, resolve
from fixes       import fixes_by_file
from records     import Width
from runner      import Runner
from stage       import Stage


class Sweep:
    """
    One corpus, the interpreter owning it, the binary formatting it, and the
    stage and runner a sweep of it works through.
    """

    def __init__(self, binary: str, python: str, target: str):
        self.binary = binary

        self.corpus, self.version = interpreter(python)

        self.only   = resolve(self.corpus, target)
        self.stage  = Stage(self.corpus)
        self.runner = Runner(python, self.stage)

    def __str__(self) -> str:
        """
        The header naming the corpus, binary, interpreter, and stage.
        """
        return (
            f"corpus      {self.corpus}\n"
            f"binary      {self.binary}\n"
            f"interpreter {self.runner.python} ({self.version})\n"
            f"stage       {self.stage.root}"
        )

    def sweep(self, width: int | None) -> Width:
        """
        Format a copy of the corpus at `width`, run every module the
        formatter rewrote from both trees, confirm each break, and attribute
        it.
        """
        label        = "default" if width is None else str(width)
        formatted    = self.stage.copy(f"formatted-{label}", width)
        records, log = format_tree(self.binary, formatted)
        (self.stage.root / f"format-{label}.log").write_text(log, encoding="utf-8")

        modules = candidates(formatted, self.only, self.stage.original)
        suspects, comparable, unmeasured = compare(
            after   = self.runner.outcomes(modules, formatted),
            before  = self.runner.originals(modules),
            modules = modules
        )

        verdicts = list(
            self.runner.pool.map(
                partial(self.runner.confirm, formatted=formatted),
                suspects
            )
        )

        breaks = [brk for brk, broken in zip(suspects, verdicts) if broken]
        flaky  = [brk.module for brk, broken in zip(suspects, verdicts) if not broken]

        Attributor(
            binary    = self.binary,
            covered   = fixes_by_file(records, formatted),
            formatted = formatted,
            label     = label,
            runner    = self.runner,
            width     = width
        ).attribute(breaks)

        return Width(
            breaks     = breaks,
            candidates = len(modules),
            comparable = len(comparable),
            flaky      = flaky,
            label      = label,
            unmeasured = unmeasured
        )

