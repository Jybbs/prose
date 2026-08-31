"""
The records one sweep leaves, meaning what a run of a module left behind, a
module the rewrite breaks, and one width's tallies and findings.
"""

from dataclasses import dataclass, field

type Frame = tuple[str, int | None]


@dataclass(frozen=True)
class Outcome:
    """
    What one run of a module left behind. `kind` is `ok`, `raised`,
    `timeout`, or `unmeasured`, `error` reads as the predicate of a sentence
    naming the module, `name` is what a raised run could not find, `loaded`
    names every module the run took from a tree, and `names` and `constants`
    hold the namespace an `ok` run bound.
    """

    kind      : str
    constants : tuple      = ()
    error     : str        = ""
    frames    : tuple      = ()
    loaded    : tuple      = ()
    name      : str | None = None
    names     : tuple      = ()


@dataclass
class Break:
    """
    A module the rewrite breaks, with what both trees left behind and the
    frame and rules the run attributed it to.
    """

    formatted   : Outcome
    module      : str
    name        : str | None
    original    : Outcome
    reason      : str
    attribution : str       = ""
    frame       : Frame     = ("", None)
    hunk        : list[str] = field(default_factory=list)

    @property
    def key(self) -> tuple[str, str]:
        """
        The file its frame names and its reason, which is what a baseline
        carries per break.
        """
        return self.frame[0], self.reason

    @property
    def loaded(self) -> tuple[str, ...]:
        """
        The modules the formatted run loaded from its tree, the module
        itself where the run recorded none.
        """
        return self.formatted.loaded or (self.module,)


@dataclass(frozen=True)
class Width:
    """
    One width's tallies and findings.
    """

    breaks     : list[Break]
    candidates : int
    comparable : int
    flaky      : list[str]
    label      : str
    unmeasured : list[str]

    @property
    def uncomparable(self) -> int:
        """
        The candidates the original tree did not run cleanly, the unmeasured
        ones aside.
        """
        return self.candidates - self.comparable - len(self.unmeasured)
