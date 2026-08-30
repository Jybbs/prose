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
        modules = candidates(self.stage.original, formatted, self.only)
        before  = self.runner.originals(modules)
        after   = self.runner.outcomes(formatted, modules)
        suspects, comparable, unmeasured = compare(modules, before, after)
        confirm  = partial(self.runner.confirm, formatted)
        verdicts = list(self.runner.pool.map(confirm, suspects))
        breaks   = list(compress(suspects, verdicts))
        flaky    = [brk.module for brk in compress(suspects, map(not_, verdicts))]
        covered  = fixes_by_file(records, formatted)
        blame = Attributor(self.runner, self.binary, formatted, covered, width, label)
        blame.attribute(breaks)
        return Width(label, len(modules), len(comparable), breaks, flaky, unmeasured)


def compare(
    modules : list[str],
    before  : dict[str, Outcome],
    after   : dict[str, Outcome]
) -> tuple[list[Break], list[str], list[str]]:
    """
    Return the breaks among `modules` between what the original tree left
    in `before` and the formatted tree in `after`, the modules comparable at
    all, and the ones a run left unmeasured.
    """
    unmeasured = [
        module
        for module in modules
        if "unmeasured" in (before[module].kind, after[module].kind)
    ]
    comparable = [
        module
        for module in modules
        if before[module].kind == "ok" and after[module].kind != "unmeasured"
    ]
    suspects = [
        Break(module, *differs, before[module], after[module])
        for module in comparable
        if (differs := divergence(before[module], after[module]))
    ]
    return suspects, comparable, unmeasured
