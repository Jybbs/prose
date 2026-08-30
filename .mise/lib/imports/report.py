"""
Render one width's findings, the breaks grouped by the frame and rules they
share, each group with the hunk around the row it names.
"""

from collections     import defaultdict
from collections.abc import Callable
from difflib         import SequenceMatcher

from records         import Break, Width

CONTEXT = 3
SHOWN   = 30


def hunk(pairs: SequenceMatcher, row: int | None) -> list[str]:
    """
    Return the diff hunk of `pairs` holding row `row` of its second
    sequence, the nearest hunk where none holds it and the first where the
    row is unknown, as unified-diff lines cut to `CONTEXT` lines either side
    of the row, the rows between the hunk and `row` appended where the hunk
    ends short of it.
    """
    groups = list(pairs.get_grouped_opcodes(CONTEXT))
    if not groups:
        return []
    at = None if row is None else row - 1

    def distance(group: list[tuple]) -> int:
        first, last = group[0][3], group[-1][4]
        if at is None or first <= at < last:
            return 0
        return min(abs(at - first), abs(at - last))

    group          = min(groups, key=distance)
    i1, i2, j1, j2 = group[0][1], group[-1][2], group[0][3], group[-1][4]
    shown          = [(None, f"@@ -{i1 + 1},{i2 - i1} +{j1 + 1},{j2 - j1} @@")]
    for tag, a1, a2, b1, b2 in group:
        if tag in ("delete", "replace"):
            shown += [(None, f"-{line}") for line in pairs.a[a1:a2]]
        if tag != "delete":
            mark   = " " if tag == "equal" else "+"
            rows   = pairs.b[b1:b2]
            shown += [(b1 + k, f"{mark}{line}") for k, line in enumerate(rows)]
    if at is not None and j2 <= at:
        gap = range(max(j2, at - CONTEXT), at + 1)
        if gap.start > j2:
            shown.append((None, "..."))
        shown += [(k, f" {pairs.b[k]}") for k in gap]
    if at is not None and at < j1:
        gap  = range(max(0, at - CONTEXT), min(j1, at + CONTEXT + 1))
        lead = [(k, f" {pairs.b[k]}") for k in gap]
        if gap.stop < j1:
            lead.append((None, "..."))
        shown[1:1] = lead
    found     = (k for k, (seen, _) in enumerate(shown) if seen == at)
    index     = 1 + CONTEXT if at is None else next(found, 1 + CONTEXT)
    low, high = max(1, index - CONTEXT), index + CONTEXT + 1
    lines     = [line for _, line in shown]
    return [
        lines[0],
        *(["..."] if low > 1 else []),
        *lines[low:high],
        *(["..."] if high < len(lines) else [])
    ]


def render(reproduce: Callable[[str], str], found: Width, carried: set[str]) -> str:
    """
    Return the report of one width's findings, `carried` naming the breaks
    the baseline already holds, the breaks grouped by the frame and rules
    they share.
    """
    timing       = [brk for brk in found.breaks if brk.formatted.kind == "timeout"]
    raising      = [brk for brk in found.breaks if brk.formatted.kind != "timeout"]
    frames       = {(brk.frame, brk.attribution) for brk in found.breaks}
    uncomparable = "unmeasured" if found.unmeasured else found.uncomparable
    lines        = [
        f"  candidates   {found.candidates:>5}",
        f"  comparable   {found.comparable:>5}",
        f"  uncomparable {uncomparable:>5}",
        f"  breaks       {len(found.breaks):>5}",
        f"  frames       {len(frames):>5}",
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
        for (frame, attribution), members in groups.items():
            lines += rendered_group(
                attribution = attribution,
                carried     = carried,
                frame       = frame,
                members     = members,
                reproduce   = reproduce
            )
    if found.flaky:
        lines += ["", f"  flaky, a second run disagreed ({len(found.flaky)}):"]
        lines += [f"    {module}" for module in found.flaky]
    if found.unmeasured:
        lines += ["", f"  unmeasured, a run left no record ({len(found.unmeasured)}):"]
        lines += [f"    {module}" for module in found.unmeasured]
    return "\n".join(lines)


def rendered_group(
    reproduce   : Callable[[str], str],
    members     : list[Break],
    frame       : tuple[str, int | None],
    attribution : str,
    carried     : set[str]
) -> list[str]:
    """
    Return the report of the breaks `members` sharing `frame` and
    `attribution`, the hunk once, a reason every member shares once, the
    modules the baseline does not carry ahead of the ones it does, and up to
    `SHOWN` modules named with the rest counted.
    """
    file, row = frame
    where     = f"{file}:{row}" if row else file
    reasons   = {brk.reason for brk in members}
    shared    = members[0].reason if len(reasons) == 1 else ""
    members   = sorted(members, key=lambda brk: (brk.module in carried, brk.module))
    lines     = [f"    {where} {attribution}"]
    if shared:
        lines.append(f"      each {shared}")
    lines += [f"      {line}" for line in members[0].hunk]
    for brk in members[:SHOWN]:
        reason   = "" if shared else f" {brk.reason}"
        carriage = ", carried by the baseline" if brk.module in carried else ""
        lines.append(f"      {brk.module}{reason}{carriage}")
    if len(members) > SHOWN:
        lines.append(f"      ... and {len(members) - SHOWN} more")
    lines.append(f"      reproduce with {reproduce(members[0].module)}")
    return lines
