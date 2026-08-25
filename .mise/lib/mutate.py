#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Write parseable mutations of a corpus, one subdirectory per mutation.

    commented   A comment line above a sample of statements.
    crlf        Every line ending rewritten to CRLF.
    shuffled    Top-level statements reordered, each keeping its source.
    suppressed  A `# prose: off` region and a logical-line `# prose: skip`.
    widened     Identifiers lengthened or shortened.

Usage: mutate.py <corpus> <destination> [budget-seconds] [seed]

The walk stops once the budget runs out, leaving the pass to cost what it
is given rather than what the corpus is worth. Every variant compiles
before it lands, so a mutation the grammar rejects is dropped rather than
reaching the formatter as a defect it never had.
"""

from ast         import ImportFrom, Module, parse, stmt, walk
from collections import defaultdict
from io          import StringIO
from keyword     import iskeyword, issoftkeyword
from pathlib     import Path
from random      import Random
from sys         import argv
from time        import monotonic
from token       import NAME
from tokenize    import TokenError, generate_tokens
from warnings    import filterwarnings

COMMENTS = 8

filterwarnings("ignore", category=SyntaxWarning)


def argument(index: int, fallback: str) -> str:
    """
    Return the `index` positional argument, or `fallback` where the
    invocation left it off.
    """
    return argv[index] if len(argv) > index else fallback


def commented(text: str, rng: Random) -> str | None:
    """
    Return `text` with a comment line inserted above a sample of its
    statements, each at that statement's own indent. `None` when the
    module carries no statement.
    """
    rows = statement_rows(text)
    if not rows:
        return None
    lines = text.splitlines(keepends=True)
    for row in sorted(rng.sample(rows, k=min(len(rows), COMMENTS)), reverse=True):
        lines.insert(row - 1, f"{indent_of(lines[row - 1])}# probe\n")
    return "".join(lines)


def crlf(text: str, _rng: Random) -> str | None:
    """
    Return `text` with every line ending rewritten to CRLF. `None` when it
    carries no line ending to rewrite.
    """
    if "\n" not in text:
        return None
    return text.replace("\r\n", "\n").replace("\n", "\r\n")


def indent_of(line: str) -> str:
    """
    Return the leading whitespace of `line`.
    """
    return line[: len(line) - len(line.lstrip())]


def is_future(node: stmt) -> bool:
    """
    Report whether `node` is a `from __future__ import ...` statement,
    which the grammar admits only ahead of every other statement.
    """
    return isinstance(node, ImportFrom) and node.module == "__future__"


def is_reserved(name: str) -> bool:
    """
    Report whether `name` is a keyword or a soft keyword, which the
    grammar reads as one rather than as an identifier.
    """
    return iskeyword(name) or issoftkeyword(name)


def parsed(text: str) -> Module | None:
    """
    Return the module `text` parses to, or `None` when it does not parse.
    """
    try:
        return parse(text)
    except (SyntaxError, ValueError):
        return None


def rename_map(tokens: list, rng: Random) -> dict[str, str]:
    """
    Return a respelling for a sample of the identifiers `tokens` carries,
    empty when none is renameable.
    """
    names = sorted({
        token.string
        for token in tokens
        if token.type == NAME and not is_reserved(token.string)
    })
    if not names:
        return {}
    renames = {}
    for name in rng.sample(names, k=max(1, len(names) // 3)):
        if candidate := renamed(name, rng):
            renames[name] = candidate
    return renames


def renamed(name: str, rng: Random) -> str | None:
    """
    Return a wider or narrower spelling of `name`, or `None` where the
    result is not an identifier the grammar reads as one.
    """
    if rng.random() < 0.5:
        candidate = name + "_w" * rng.randint(1, 3)
    else:
        candidate = name[: max(1, len(name) // 2)]
    if candidate == name or not candidate.isidentifier():
        return None
    return None if is_reserved(candidate) else candidate


def shuffled(text: str, rng: Random) -> str | None:
    """
    Return `text` with its top-level statements reordered, each keeping
    the lines it owns. A `__future__` import holds its seat ahead of the
    shuffle. `None` when fewer than two statements are free to move.
    """
    module = parsed(text)
    if module is None:
        return None
    lines = text.splitlines(keepends=True)
    pinned, movable, start = [], [], 0
    for node in module.body:
        chunk = terminated("".join(lines[start : node.end_lineno]))
        (pinned if is_future(node) else movable).append(chunk)
        start = node.end_lineno
    if len(movable) < 2:
        return None
    rng.shuffle(movable)
    return "".join(pinned + movable + lines[start:])


def statement_rows(text: str) -> list[int]:
    """
    Return the 1-based rows a statement starts on, which are the rows a
    comment line may precede. Empty when `text` does not parse.
    """
    module = parsed(text)
    if module is None:
        return []
    return sorted({node.lineno for node in walk(module) if isinstance(node, stmt)})


def suppressed(text: str, rng: Random) -> str | None:
    """
    Return `text` with a `# prose: off` region wrapped around one
    top-level statement and a `# prose: skip` closing a statement's
    logical line. `None` when the module carries no top-level statement.
    """
    module = parsed(text)
    if module is None or not module.body:
        return None
    lines = text.splitlines(keepends=True)
    row = rng.choice(module.body).end_lineno - 1
    lines[row] = lines[row].rstrip("\r\n") + "  # prose: skip\n"
    node = rng.choice(module.body)
    indent = indent_of(lines[node.lineno - 1])
    lines.insert(node.end_lineno, f"{indent}# prose: on\n")
    lines.insert(node.lineno - 1, f"{indent}# prose: off\n")
    return "".join(lines)


def terminated(chunk: str) -> str:
    """
    Return `chunk` carrying a trailing newline, so a reordered statement
    never joins the line beneath it.
    """
    return chunk if chunk.endswith("\n") else f"{chunk}\n"


def variants(text: str, rng: Random) -> dict[str, str]:
    """
    Return each mutation's rendering of `text`, dropping the ones that do
    not apply and the ones the grammar rejects.
    """
    built = {}
    for mutation in MUTATIONS:
        candidate = mutation(text, rng)
        if candidate is None:
            continue
        try:
            compile(candidate, mutation.__name__, "exec")
        except (SyntaxError, ValueError):
            continue
        built[mutation.__name__] = candidate
    return built


def widened(text: str, rng: Random) -> str | None:
    """
    Return `text` with a sample of its identifiers lengthened or
    shortened, shifting every column their width feeds. `None` when it
    does not tokenize or carries no renameable name.
    """
    try:
        tokens = list(generate_tokens(StringIO(text).readline))
    except (IndentationError, SyntaxError, TokenError, ValueError):
        return None
    renames = rename_map(tokens, rng)
    if not renames:
        return None
    edits = defaultdict(list)
    for token in tokens:
        if token.type == NAME and token.string in renames:
            row, column = token.start
            edits[row].append((column, token.end[1], renames[token.string]))
    lines = text.splitlines(keepends=True)
    for row, spans in edits.items():
        line = lines[row - 1]
        for start, end, replacement in sorted(spans, reverse=True):
            line = line[:start] + replacement + line[end:]
        lines[row - 1] = line
    return "".join(lines)


MUTATIONS = (commented, crlf, shuffled, suppressed, widened)


if __name__ == "__main__":

    corpus, destination = Path(argv[1]), Path(argv[2])
    budget   = float(argument(3, "60"))
    rng      = Random(int(argument(4, "0")))
    deadline = monotonic() + budget
    written  = 0

    for path in sorted(corpus.rglob("*.py")):
        if monotonic() >= deadline:
            break
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for name, variant in variants(text, rng).items():
            target = destination / name / path.relative_to(corpus)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(variant, encoding="utf-8", newline="")
            written += 1

    print(f"{written} variants written")
