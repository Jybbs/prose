"""
The diff between a module's two versions, meaning the text read once per
path, the line matcher built once per pair, the original rows a formatted
row came from, and the hunk around a row.
"""

from difflib   import SequenceMatcher
from functools import cache
from pathlib   import Path

CONTEXT = 3


def hunk(pairs: SequenceMatcher, row: int | None, name: str = "") -> list[str]:
    """
    Return the unified-diff lines of `pairs` cut to `CONTEXT` lines either
    side of row `row` of its second sequence, or where the row is unknown
    of the first changed line naming `name`, else the first changed line, an
    ellipsis marking each cut.
    """
    shown = []
    for tag, i1, i2, j1, j2 in pairs.get_opcodes():
        if tag in ("delete", "replace"):
            shown += [(None, f"-{line}") for line in pairs.a[i1:i2]]
        if tag != "delete":
            mark   = " " if tag == "equal" else "+"
            shown += [(j1 + k, f"{mark}{line}") for k, line in enumerate(
                pairs.b[j1:j2]
            )]
    if row is None:
        changed = [k for k, (_, line) in enumerate(shown) if line[0] != " "]
        index   = next(iter([k for k in changed if name in shown[k][1]] or changed), 0)
    else:
        index = next((k for k, (seen, _) in enumerate(shown) if seen == row - 1), 0)
    low, high = max(0, index - CONTEXT), index + CONTEXT + 1
    return [
        *(["..."] if low else []),
        *(line for _, line in shown[low:high]),
        *(["..."] if high < len(shown) else [])
    ]


def mapped_rows(pairs: SequenceMatcher, row: int) -> range:
    """
    Return the rows of `pairs`'s first sequence that row `row` of its second
    came from, one row for an equal block, every row of the block that
    rewrote it, and the row an insertion landed at.
    """
    for tag, i1, i2, j1, j2 in pairs.get_opcodes():
        if j1 <= row - 1 < j2:
            if tag == "equal":
                return range(landed := i1 + row - j1, landed + 1)
            return range(i1 + 1, max(i1 + 1, i2) + 1)
    return range(0)


@cache
def pairing(after: Path, before: Path) -> SequenceMatcher:
    """
    Return the line matcher between the modules at `before` and `after`,
    built once per pair.
    """
    return SequenceMatcher(
        None,
        text_of(before).splitlines(),
        text_of(after).splitlines(),
        autojunk = False
    )


@cache
def text_of(path: Path) -> str:
    """
    Return the text of the module at `path`, read once per path.
    """
    return path.read_text(encoding="utf-8", errors="replace")
