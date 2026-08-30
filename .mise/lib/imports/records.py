"""
The records one sweep leaves, meaning what a run of a module left behind, a
module the rewrite breaks, and one width's tallies and findings.
"""

from dataclasses import dataclass, field


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
    error     : str        = ""
    frames    : tuple      = ()
    loaded    : tuple      = ()
    name      : str | None = None
    names     : tuple      = ()
    constants : tuple      = ()


@dataclass
class Break:
    """
    A module the rewrite breaks, with the frame and rules the run attributed
    it to.
    """

    module      : str
    reason      : str
    name        : str | None
    formatted   : Outcome
    attribution : str = ""
    frame       : tuple[str, int | None] = ("", None)
    hunk        : list[str]              = field(default_factory=list)

    @property
    def loaded(self) -> tuple[str, ...]:
        """
        The modules the formatted run loaded from its tree, the module
        itself where the run recorded none.
        """
        return self.formatted.loaded or (self.module,)


@dataclass
class Width:
    """
    One width's tallies and findings.
    """

    label        : str
    candidates   : int
    comparable   : int         = 0
    uncomparable : int         = 0
    breaks       : list[Break] = field(default_factory=list)
    flaky        : list[str]   = field(default_factory=list)
    unmeasured   : list[str]   = field(default_factory=list)
