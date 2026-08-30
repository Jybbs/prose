"""
Which recorded fix reached a row or a binding, meaning the format run's
records grouped by file, the rows an edit rewrote, and the span one fix's
edits reach as written and as they leave it.
"""

from collections import defaultdict
from itertools   import accumulate
from pathlib     import Path
from re          import escape, search

Fixes = dict[str, list[tuple[str, list[dict]]]]


def drops(text: str, edits: list[dict], name: str) -> bool:
    """
    Report whether `edits`, one fix's edits over `text` together, take
    `name` out of the span they reach.
    """
    was, now = rewritten(text, edits)
    word     = rf"\b{escape(name)}\b"
    return bool(search(word, was)) and not search(word, now)


def edit_rows(edit: dict) -> range:
    """
    Return the original rows `edit` rewrote, an end at column 1 closing on
    the row above it.
    """
    start, end = edit["location"]["row"], edit["end_location"]["row"]
    if end > start and edit["end_location"]["column"] == 1:
        end -= 1
    return range(start, end + 1)


def fixes_by_file(records: list[dict], tree: Path) -> Fixes:
    """
    Return the safe-fix records grouped by the file they rewrote, relative
    to `tree`, each as its rule slug and edits.
    """
    by_file = defaultdict(list)
    for record in records:
        if (fix := record.get("fix")) and fix["applicability"] == "safe":
            file = Path(record["filename"]).relative_to(tree).as_posix()
            by_file[file].append((record["code"], fix["edits"]))
    return by_file


def reaches(edits: list[dict], rows: range, text: str = "") -> bool:
    """
    Report whether one of `edits` rewrote one of the original `rows` or
    wrote a line reading `text`.
    """
    return any(
        (span := edit_rows(edit)).start < rows.stop and rows.start < span.stop
        or text and text in map(str.strip, edit["content"].splitlines())
        for edit in edits
    )


def rewritten(text: str, edits: list[dict]) -> tuple[str, str]:
    """
    Return the lines of `text` one fix's `edits` reach, as written and as
    the edits leave them, each edit placed by its row and column in `text`.
    """
    starts = [0, *accumulate(len(line) + 1 for line in text.split("\n"))]

    def offset(place: dict) -> int:
        return starts[place["row"] - 1] + place["column"] - 1

    spans = sorted(
        (offset(edit["location"]), offset(edit["end_location"]), edit["content"])
        for edit in edits
    )
    low  = text.rfind("\n", 0, spans[0][0]) + 1
    high = text.find("\n", max(end for _, end, _ in spans))
    high = len(text) if high < 0 else high
    left = text
    for start, end, content in reversed(spans):
        left = f"{left[:start]}{content}{left[end:]}"
    return text[low:high], left[low:high + len(left) - len(text)]
