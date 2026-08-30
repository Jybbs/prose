"""
Render one width's findings, the breaks grouped by the frame and rules they
share, each group with the hunk around the row it names and the command
reproducing one of its modules alone.
"""

from collections import defaultdict
from os          import environ
from pathlib     import Path
from shlex       import quote

from records import Break, Width

KNOBS = ("PROSE_IMPORTS_PROFILE", "PROSE_IMPORTS_PYTHON", "PROSE_IMPORTS_TIMEOUT")
SHOWN = 30


def render(found: Width, carried: set[str], corpus: Path) -> str:
    """
    Return the report of one width's findings, `carried` naming the broken
    modules the baseline already holds, the breaks grouped by the frame and
    rules they share.
    """
    timing  = [brk for brk in found.breaks if brk.formatted.kind == "timeout"]
    raising = [brk for brk in found.breaks if brk.formatted.kind != "timeout"]
    lines   = [
        f"  candidates   {found.candidates:>5}",
        f"  comparable   {found.comparable:>5}",
        f"  uncomparable {'unmeasured' if found.unmeasured else found.uncomparable:>5}",
        f"  breaks       {len(found.breaks):>5}",
        f"  timeouts     {len(timing):>5}",
        f"  flaky        {len(found.flaky):>5}"
    ]
    if carried:
        lines.append(f"  carried      {len(carried):>5}")
    for heading, listed in (("raises or rebinds", raising), ("times out", timing)):
        if not listed:
            continue
        groups = defaultdict(list)
        for brk in listed:
            groups[(brk.frame, brk.attribution)].append(brk)
        lines += ["", f"  {heading} ({len(listed)} modules at {len(groups)} frames):"]
        for members in groups.values():
            lines += rendered_group(
                carried = carried,
                corpus  = corpus,
                label   = found.label,
                members = members
            )
    for heading, listed in (
        ("flaky, a second run disagreed", found.flaky),
        ("unmeasured, a run left no record", found.unmeasured)
    ):
        if listed:
            lines += [
                "",
                f"  {heading} ({len(listed)}):",
                *(f"    {module}" for module in listed)
            ]
    return "\n".join(lines)


def rendered_group(
    members : list[Break],
    carried : set[str],
    corpus  : Path,
    label   : str
) -> list[str]:
    """
    Return the report of the breaks `members` sharing a frame and
    attribution, the hunk once, a reason every member shares once, the
    modules the baseline does not carry ahead of the ones it does, and up to
    `SHOWN` modules named with the rest counted.
    """
    file, row = members[0].frame
    where     = f"{file}:{row}" if row else file
    reasons   = {brk.reason for brk in members}
    shared    = members[0].reason if len(reasons) == 1 else ""
    ordered   = sorted(members, key=lambda brk: (brk.module in carried, brk.module))
    lines     = [f"    {where} {members[0].attribution}"]
    if shared:
        lines.append(f"      each {shared}")
    lines += [f"      {line}" for line in members[0].hunk]
    for brk in ordered[:SHOWN]:
        reason   = "" if shared else f" {brk.reason}"
        carriage = ", carried by the baseline" if brk.module in carried else ""
        lines.append(f"      {brk.module}{reason}{carriage}")
    if len(ordered) > SHOWN:
        lines.append(f"      ... and {len(ordered) - SHOWN} more")
    command = reproduction(corpus, ordered[0].module, label)
    lines.append(f"      reproduce with {command}")
    return lines


def reproduction(corpus: Path, module: str, label: str) -> str:
    """
    Return the command running `module` of `corpus` alone at the width
    `label` names.
    """
    knobs = [f"{knob}={quote(environ[knob])}" for knob in KNOBS if knob in environ]
    if label != "default":
        knobs.append(f"PROSE_IMPORTS_WIDTHS={label}")
    return " ".join([*knobs, "mise", "run", "imports", quote(str(corpus / module))])
