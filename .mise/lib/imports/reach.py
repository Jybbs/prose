"""
Which recorded fix reached a row or a binding, meaning the format run's
records grouped by file, the spans a fix rewrote, the bindings a module
makes, and the line matcher between a module's two versions.
"""

from ast import (
    AnnAssign, Assign, AsyncFor, AsyncFunctionDef, AsyncWith, ClassDef, For,
    FunctionDef, Import, ImportFrom, List, Name, Starred, Tuple, With, expr, parse,
    stmt,
)
from collections     import defaultdict
from collections.abc import Iterator
from difflib         import SequenceMatcher
from functools       import cache
from itertools       import accumulate
from pathlib         import Path
from re              import escape, search


@cache
def bindings(path: Path) -> dict[str, range]:
    """
    Return the rows of the first module-level statement binding each name in
    the module at `path`, one inside a compound statement counting and one
    inside a function or class not, empty where the module does not parse.
    """
    try:
        module = parse(text_of(path))
    except (SyntaxError, ValueError):
        return {}
    rows = {}
    for node in statements(module.body):
        for name in bound(node):
            rows.setdefault(name, header_rows(node))
    return rows


def bound(node: stmt) -> list[str]:
    """
    Return the names a module-level statement `node` binds.
    """
    match node:
        case FunctionDef() | AsyncFunctionDef() | ClassDef(): return [node.name]
        case Import() | ImportFrom():
            return [
                (alias.asname or alias.name).split(".")[0]
                for alias in node.names
                if alias.name != "*"
            ]
        case Assign():
            return [name for target in node.targets for name in targets(target)]
        case AnnAssign()        : return targets(node.target)
        case For() | AsyncFor() : return targets(node.target)
        case With() | AsyncWith():
            return [
                name
                for item in node.items
                if item.optional_vars
                for name in targets(item.optional_vars)
            ]
    return []


def covers(edit: dict, rows: range, text: str) -> bool:
    """
    Report whether `edit` rewrote one of the original `rows` or wrote a line
    reading `text`.
    """
    span = edit_rows(edit)
    if max(span.start, rows.start) < min(span.stop, rows.stop):
        return True
    return bool(text) and text in map(str.strip, edit["content"].splitlines())


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


def fixes_by_file(records: list[dict], tree: Path) -> dict:
    """
    Return the safe-fix records grouped by the file they rewrote, relative
    to `tree`, each as its rule slug and edits.
    """
    by_file = defaultdict(list)
    for record in records:
        fix = record.get("fix")
        if fix and fix["applicability"] == "safe":
            file = Path(record["filename"]).relative_to(tree).as_posix()
            by_file[file].append((record["code"], fix["edits"]))
    return by_file


def header_rows(node: stmt) -> range:
    """
    Return the rows a statement `node` binds its names on, the header alone
    for a function or class.
    """
    if isinstance(node, (AsyncFunctionDef, ClassDef, FunctionDef)):
        return range(node.lineno, max(node.body[0].lineno, node.lineno + 1))
    return range(node.lineno, node.end_lineno + 1)


@cache
def lines_of(path: Path) -> list[str]:
    """
    Return the lines of the module at `path`, read once per path.
    """
    return text_of(path).splitlines()


def mapped_rows(pairs: SequenceMatcher, row: int, back: bool = True) -> range:
    """
    Return the rows of `pairs`'s first sequence that row `row` of its second
    came from, or of its second that row `row` of its first went to where
    `back` is false, one row for an equal block, every row of the block that
    rewrote it, and the row an insertion landed at.
    """
    for tag, i1, i2, j1, j2 in pairs.get_opcodes():
        (a1, a2), (b1, b2) = ((i1, i2), (j1, j2)) if back else ((j1, j2), (i1, i2))
        if b1 <= row - 1 < b2:
            if tag == "equal":
                return range(a1 + row - b1, a1 + row - b1 + 1)
            return range(a1 + 1, max(a1 + 1, a2) + 1)
    return range(0)


@cache
def pairing(before: Path, after: Path) -> SequenceMatcher:
    """
    Return the line matcher between the modules at `before` and `after`,
    built once per pair.
    """
    return SequenceMatcher(None, lines_of(before), lines_of(after), autojunk=False)


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


def statements(body: list[stmt]) -> Iterator[stmt]:
    """
    Yield every statement in `body` and in the compound statements it holds,
    without entering a function or class.
    """
    for node in body:
        yield node
        if isinstance(node, (AsyncFunctionDef, ClassDef, FunctionDef)):
            continue
        for held in ("body", "finalbody", "orelse"):
            yield from statements(getattr(node, held, []))
        for handler in getattr(node, "handlers", []):
            yield from statements(handler.body)
        for case in getattr(node, "cases", []):
            yield from statements(case.body)


def targets(target: expr) -> list[str]:
    """
    Return the names an assignment `target` binds, walking a tuple, list, or
    starred target.
    """
    match target:
        case Name(): return [target.id]
        case Tuple() | List():
            return [name for elt in target.elts for name in targets(elt)]
        case Starred(): return targets(target.value)
    return []


@cache
def text_of(path: Path) -> str:
    """
    Return the text of the module at `path`, read once per path.
    """
    return path.read_text(encoding="utf-8", errors="replace")
