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

The walk stops once the budget runs out, and every variant compiles before
it lands.
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


class Mutator:
    """
    The mutations one seeded run writes.

    Attributes:
        rng: The seeded source every sample and shuffle draws from.
    """

    def __init__(self, seed: int):
        """
        Args:
            seed: The seed this run's sampling starts from.
        """
        self.rng = Random(seed)

    def commented(self, text: str) -> str | None:
        """
        Return `text` with a comment line inserted above a sample of its
        statements, each at that statement's own indent. `None` when the
        module carries no statement.

        Args:
            text: The module source to comment.
        """
        rows = statement_rows(text)

        if not rows:
            return None

        lines = text.splitlines(keepends=True)

        for row in sorted(
            self.rng.sample(rows, k=min(len(rows), COMMENTS)),
            reverse = True
        ):
            lines.insert(row - 1, f"{indent_of(lines[row - 1])}# probe\n")

        return "".join(lines)

    def crlf(self, text: str) -> str | None:
        """
        Return `text` with every line ending rewritten to CRLF. `None` when
        it carries no line ending to rewrite.

        Args:
            text: The module source to rewrite.
        """
        if "\n" not in text:
            return None

        return text.replace("\r\n", "\n").replace("\n", "\r\n")

    def renames(self, tokens: list) -> dict[str, str]:
        """
        Return a respelling for a sample of the identifiers `tokens`
        carries, empty when none is renameable.

        Args:
            tokens: The tokens of the module being widened.
        """
        names = sorted(
            {
                token.string
                for token in tokens
                if token.type == NAME and not is_reserved(token.string)
            }
        )

        if not names:
            return {}

        return {
            name: candidate
            for name, candidate in (
                (name, self.respelt(name))
                for name in self.rng.sample(names, k=max(1, len(names) // 3))
            )
            if candidate is not None
        }

    def respelt(self, name: str) -> str | None:
        """
        Return a wider or narrower spelling of `name`, or `None` where the
        result is not an identifier the grammar reads as one.

        Args:
            name: The identifier to respell.
        """
        candidate = (
            name + "_w" * self.rng.randint(1, 3)
            if self.rng.random() < 0.5
            else name[: max(1, len(name) // 2)]
        )

        if candidate == name or not candidate.isidentifier():
            return None

        return None if is_reserved(candidate) else candidate

    def shuffled(self, text: str) -> str | None:
        """
        Return `text` with its top-level statements reordered, each keeping
        the lines it owns. A `__future__` import holds its seat ahead of the
        shuffle. `None` when fewer than two statements are free to move.

        Args:
            text: The module source to reorder.
        """
        module = parsed(text)

        if module is None:
            return None

        lines = text.splitlines(keepends=True)
        pinned, movable, start = [], [], 0

        for node in module.body:
            (pinned if is_future(node) else movable).append(
                terminated("".join(lines[start : node.end_lineno]))
            )
            start = node.end_lineno

        if len(movable) < 2:
            return None

        self.rng.shuffle(movable)

        return "".join(pinned + movable + lines[start:])

    def suppressed(self, text: str) -> str | None:
        """
        Return `text` with a `# prose: off` region wrapped around one
        top-level statement and a `# prose: skip` closing a statement's
        logical line. `None` when the module carries no top-level statement.

        Args:
            text: The module source to suppress within.
        """
        module = parsed(text)

        if module is None or not module.body:
            return None

        lines      = text.splitlines(keepends=True)
        row        = self.rng.choice(module.body).end_lineno - 1
        lines[row] = lines[row].rstrip("\r\n") + "  # prose: skip\n"

        node   = self.rng.choice(module.body)
        indent = indent_of(lines[node.lineno - 1])

        lines.insert(node.end_lineno, f"{indent}# prose: on\n")
        lines.insert(node.lineno - 1, f"{indent}# prose: off\n")

        return "".join(lines)

    def variants(self, text: str) -> dict[str, str]:
        """
        Return each mutation's rendering of `text`, dropping the ones that
        do not apply and the ones the grammar rejects.

        Args:
            text: The module source to mutate.
        """
        built = {}

        for mutation in (
            self.commented, self.crlf, self.shuffled, self.suppressed, self.widened
        ):
            candidate = mutation(text)

            if candidate is None:
                continue

            try:
                compile(candidate, mutation.__name__, "exec")
            except (SyntaxError, ValueError):
                continue

            built[mutation.__name__] = candidate

        return built

    def widened(self, text: str) -> str | None:
        """
        Return `text` with a sample of its identifiers lengthened or
        shortened, shifting every column their width feeds. `None` when it
        does not tokenize or carries no renameable name.

        Args:
            text: The module source to respell within.
        """
        try:
            tokens = list(generate_tokens(StringIO(text).readline))
        except (IndentationError, SyntaxError, TokenError, ValueError):
            return None

        renames = self.renames(tokens)

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


def indent_of(line: str) -> str:
    """
    Return the leading whitespace of `line`.

    Args:
        line: The line to measure.
    """
    return line[: len(line) - len(line.lstrip())]


def is_future(node: stmt) -> bool:
    """
    Report whether `node` is a `from __future__ import ...` statement, which
    the grammar admits only ahead of every other statement.

    Args:
        node: The top-level statement to test.
    """
    return isinstance(node, ImportFrom) and node.module == "__future__"


def is_reserved(name: str) -> bool:
    """
    Report whether `name` is a keyword or a soft keyword, which the grammar
    reads as one rather than as an identifier.

    Args:
        name: The candidate identifier.
    """
    return iskeyword(name) or issoftkeyword(name)


def main(corpus: Path, deadline: float, destination: Path, mutator: Mutator):
    """
    Write every mutation of every module the deadline reaches.

    Args:
        corpus      : The tree of modules to mutate.
        deadline    : The monotonic time the walk stops at.
        destination : The tree each mutation's variants are written under.
        mutator     : The seeded mutator each variant is drawn from.
    """
    written = 0

    for path in sorted(corpus.rglob("*.py")):
        if monotonic() >= deadline:
            break

        text = read(path)

        if text is None:
            continue

        for name, variant in mutator.variants(text).items():
            target = destination / name / path.relative_to(corpus)
            target.parent.mkdir(exist_ok=True, parents=True)
            target.write_text(variant, encoding="utf-8", newline="")
            written += 1

    print(f"{written} variants written")


def read(path: Path) -> str | None:
    """
    Return the text of `path`, or `None` where it does not read as UTF-8.

    Args:
        path: The file to read.
    """
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None


def parsed(text: str) -> Module | None:
    """
    Return the module `text` parses to, or `None` when it does not parse.

    Args:
        text: The module source to parse.
    """
    try:
        return parse(text)
    except (SyntaxError, ValueError):
        return None


def statement_rows(text: str) -> list[int]:
    """
    Return the 1-based rows a statement starts on, which are the rows a
    comment line may precede. Empty when `text` does not parse.

    Args:
        text: The module source to scan.
    """
    module = parsed(text)

    if module is None:
        return []

    return sorted({node.lineno for node in walk(module) if isinstance(node, stmt)})


def terminated(chunk: str) -> str:
    """
    Return `chunk` carrying a trailing newline, so a reordered statement
    never joins the line beneath it.

    Args:
        chunk: The statement source to terminate.
    """
    return chunk if chunk.endswith("\n") else f"{chunk}\n"


main(
    corpus      = Path(argv[1]),
    deadline    = monotonic() + (float(argv[3]) if len(argv) > 3 else 60.0),
    destination = Path(argv[2]),
    mutator     = Mutator(int(argv[4]) if len(argv) > 4 else 0)
)
