"""
Attributing a break, meaning the frame it raises in, the rules whose
recorded fixes reach that frame or drop the binding it hinges on, and the
rules reproducing it alone where no record does.
"""

from collections.abc import Callable
from dataclasses     import dataclass
from itertools       import compress
from pathlib         import Path

from binary     import format_tree, rules
from bindings   import bindings
from comparison import divergence
from diff       import hunk, mapped_rows, pairing, text_of
from fixes      import Fixes, drops, reaches
from records    import Break, Frame
from runner     import Runner


@dataclass
class Attributor:
    """
    One width's formatted tree and the fixes its format run recorded, with
    the runner and binary an attribution runs and formats through.
    """

    binary    : str
    covered   : Fixes
    formatted : Path
    label     : str
    runner    : Runner
    width     : int | None

    def alone(self, brk: Break) -> str:
        """
        Return the rules reproducing `brk` for the same reason when every
        module its run loaded from the formatted tree is formatted under
        each one alone, joined in pipeline order.
        """
        stage, width = self.runner.stage, self.width

        def reproduces(slug: str) -> bool:
            tree = stage.overlay(brk.loaded, self.label, brk.module, slug, width)
            format_tree(self.binary, tree, slug)
            found = divergence(
                self.runner.execute(brk.module, [tree, stage.original]),
                brk.original
            )
            return found is not None and found[0] == brk.reason

        listed = rules(self.binary)
        return ", ".join(compress(listed, self.runner.pool.map(reproduces, listed)))

    def attribute(self, breaks: list[Break]):
        """
        Locate each of `breaks` and explain the first of each group sharing
        a frame and reason, the rest taking its attribution and hunk.
        """
        explained = {}
        for brk in breaks:
            brk.frame = self.locate(brk)
            first     = explained.setdefault((brk.frame, brk.reason), brk)

            if first is brk:
                self.explain(brk)
            else:
                brk.attribution, brk.hunk = first.attribution, first.hunk

    def binding(self, brk: Break) -> str:
        """
        Return the clause naming where a module `brk` loaded bound the name
        it hinges on and the rules whose fixes dropped that name from the
        binding, empty where no fix did.
        """
        for module in brk.loaded:
            path = self.runner.stage.original / module
            if rows := bindings(path).get(brk.name):

                def fits(edits: list[dict]) -> bool:
                    return reaches(edits, rows) and drops(
                        edits,
                        brk.name,
                        text_of(path)
                    )

                if listed := self.fitting(module, fits):
                    return (
                        f"`{brk.name}` bound at {module}:{rows.start}, "
                        f"dropped by {listed}"
                    )

        return ""

    def explain(self, brk: Break):
        """
        Fill `brk`, its frame located, with the hunk around that frame and
        the rules the format run's records attribute it to, or the rules
        reproducing it alone where no record does.
        """
        file, row = brk.frame
        pairs     = pairing(self.formatted / file, self.runner.stage.original / file)
        clauses   = []

        if row is not None:
            rows, line = mapped_rows(pairs, row), pairs.b[row - 1].strip()
            if under := self.fitting(file, lambda edits: reaches(edits, rows, line)):
                clauses.append(f"under {under}")

        if brk.name and (clause := self.binding(brk)):
            clauses.append(clause)

        brk.attribution = ", ".join(clauses) or (
            f"reproduced by {alone} alone"
            if (alone := self.alone(brk))
            else "no single rule reproduces it"
        )

        brk.hunk = hunk(pairs, row, brk.name or "")

    def fitting(self, file: str, fits: Callable[[list[dict]], bool]) -> str:
        """
        Return the rules whose recorded fixes to `file` satisfy `fits`,
        joined in pipeline order.
        """
        hit = {slug for slug, edits in self.covered.get(file, []) if fits(edits)}

        return ", ".join(slug for slug in rules(self.binary) if slug in hit)

    def locate(self, brk: Break) -> Frame:
        """
        Return the file and row `brk` names, taking the deepest traceback
        frame under the formatted tree, otherwise the row of the formatted
        module binding the name it hinges on, and the module alone where
        neither exists.
        """
        if under := [
            (path.relative_to(self.formatted).as_posix(), line)
            for file, line in brk.formatted.frames
            if (path := Path(file)).is_relative_to(self.formatted)
        ]:
            return under[-1]

        rows = bindings(self.formatted / brk.module).get(brk.name)
        return brk.module, rows.start if rows else None
