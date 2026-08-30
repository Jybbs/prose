"""
One sweep of a corpus at a width, meaning a formatted copy, the run of every
module the formatter rewrote from both trees, and each break confirmed and
attributed.
"""

from functools import partial
from itertools import compress
from operator  import not_

from attribution import Attributor
from binary      import format_tree
from corpus      import candidates, interpreter, resolve
from fixes       import fixes_by_file
from records     import Break, Outcome, Width
from runner      import Runner, divergence
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
        breaks = list(compress(suspects, verdicts))
        Attributor(
            self.binary,
            fixes_by_file(records, formatted),
            formatted,
            label,
            self.runner,
            width
        ).attribute(breaks)
        return Width(
            breaks,
            len(modules),
            len(comparable),
            [brk.module for brk in compress(suspects, map(not_, verdicts))],
            label,
            unmeasured
        )


def compare(
    after   : dict[str, Outcome],
    before  : dict[str, Outcome],
    modules : list[str]
) -> tuple[list[Break], list[str], list[str]]:
    """
    Return the breaks among `modules` between what the original tree left
    in `before` and the formatted tree in `after`, the modules comparable at
    all, and the ones a run left unmeasured.
    """
    comparable = [
        module
        for module in modules
        if before[module].kind == "ok" and after[module].kind != "unmeasured"
    ]
    return (
        [
            Break(
                formatted = after[module],
                module    = module,
                name      = differs[1],
                original  = before[module],
                reason    = differs[0]
            )
            for module in comparable
            if (differs := divergence(after[module], before[module]))
        ],
        comparable,
        [
            module
            for module in modules
            if "unmeasured" in (before[module].kind, after[module].kind)
        ]
    )
