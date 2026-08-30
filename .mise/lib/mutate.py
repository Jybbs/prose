#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["libcst==1.9.0"]
# ///
"""
Write parseable mutations of a corpus, one subdirectory per mutation.

    commented      A comment line above a sample of statements.
    crlf           Every line ending rewritten to CRLF.
    members        Each class body's members reordered behind its docstring.
    parenthesized  Every argument of a sample of calls wrapped in parentheses.
    shuffled       Top-level statements reordered, each keeping its lines.
    suppressed     A `# prose: off` region and a logical-line `# prose: skip`.
    widened        Identifiers lengthened or shortened.

Usage: mutate.py <corpus> <destination> [budget-seconds] [seed]

Each mutation edits the module's concrete syntax tree, so a comment or a
blank line travels with the statement it leads and every byte a mutation
leaves alone round-trips. Files mutate across a process pool, each
seeded from the seed and its own path, so a variant is the same whatever
order the pool reaches the files in. The walk stops once the budget runs
out, leaving the pass to cost what it is given rather than what the
corpus is worth. Every variant compiles before it lands, so a mutation
the grammar rejects is dropped rather than reaching the formatter as a
defect it never had.
"""

from argparse           import ArgumentParser
from collections.abc    import Callable
from concurrent.futures import ProcessPoolExecutor, TimeoutError, as_completed
from keyword            import iskeyword, issoftkeyword
from pathlib            import Path
from random             import Random
from warnings           import filterwarnings

from libcst import (
    BaseCompoundStatement, BaseStatement, Call, ClassDef, Comment, ConcatenatedString, CSTLogicError,
    CSTNode, EmptyLine, Expr, ImportFrom, IndentedBlock, LeftParen, Module, Name, ParserSyntaxError,
    RightParen, SimpleStatementLine, SimpleString, SimpleWhitespace, TrailingWhitespace, parse_module,
)
from libcst.matchers import MatchIfTrue, findall, replace

SAMPLE = 8

filterwarnings("ignore", category=SyntaxWarning)


def collect(module: Module, *kinds: type) -> list[CSTNode]:
    """
    Return every node of `kinds` that `module` holds, in source order.
    """
    return findall(module, MatchIfTrue(lambda node: isinstance(node, kinds)))


def commented(module: Module, rng: Random) -> Module | None:
    """
    Return `module` with a comment line leading a sample of its statements,
    each at that statement's own indent. `None` when it holds no statement.
    """
    statements = collect(module, SimpleStatementLine, BaseCompoundStatement)
    if not statements:
        return None
    return rewrite(module, rng.sample(statements, k=min(len(statements), SAMPLE)), lambda node: led(node, "# probe"))


def crlf(module: Module, _rng: Random) -> Module | None:
    """
    Return `module` with every line ending rewritten to CRLF. `None` when
    it carries no line ending to rewrite.
    """
    if "\n" not in module.code:
        return None
    return module.with_changes(default_newline="\r\n")


def is_docstring(statement: BaseStatement) -> bool:
    """
    Report whether `statement` is a lone string expression.
    """
    return (
        isinstance(statement, SimpleStatementLine)
        and len(statement.body) == 1
        and isinstance(statement.body[0], Expr)
        and isinstance(statement.body[0].value, (ConcatenatedString, SimpleString))
    )


def is_future(statement: BaseStatement) -> bool:
    """
    Report whether `statement` is a `from __future__ import ...`, which
    the grammar admits only ahead of every other statement.
    """
    return isinstance(statement, SimpleStatementLine) and any(
        isinstance(small, ImportFrom) and isinstance(small.module, Name) and small.module.value == "__future__"
        for small in statement.body
    )


def is_reserved(name: str) -> bool:
    """
    Report whether `name` is a keyword or a soft keyword, which the
    grammar reads as one rather than as an identifier.
    """
    return iskeyword(name) or issoftkeyword(name)


def leads(index: int, statement: BaseStatement) -> bool:
    """
    Report whether `statement` is a docstring seated first.
    """
    return index == 0 and is_docstring(statement)


def led(statement: BaseStatement, comment: str, first: bool = False) -> BaseStatement:
    """
    Return `statement` with a `comment` line among its leading lines, last
    among them so it sits directly above the statement, or `first` so it
    sits directly beneath the statement before it.
    """
    line  = EmptyLine(comment=Comment(comment))
    lines = [line, *statement.leading_lines] if first else [*statement.leading_lines, line]
    return statement.with_changes(leading_lines=lines)


def mutated(path: Path, corpus: Path, destination: Path, seed: str) -> int:
    """
    Write every variant of the file at `path` under `destination`, one
    subdirectory per mutation, and return how many landed.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return 0
    relative = path.relative_to(corpus)
    written  = 0
    for name, variant in variants(text, Random(f"{seed}:{relative}")).items():
        target = destination / name / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(variant, encoding="utf-8", newline="")
        written += 1
    return written


def members(module: Module, rng: Random) -> Module | None:
    """
    Return `module` with each class body's members reordered behind its
    docstring. `None` when no class holds two members free to move.
    """
    classes = [
        node
        for node in collect(module, ClassDef)
        if isinstance(node.body, IndentedBlock) and len(partition(node.body.body, leads)[1]) >= 2
    ]
    if not classes:
        return None

    def reordered(node: ClassDef) -> ClassDef:
        pinned, movable = partition(node.body.body, leads)
        rng.shuffle(movable)
        return node.with_changes(body=node.body.with_changes(body=[*pinned, *movable]))

    return rewrite(module, classes, reordered)


def parenthesized(module: Module, rng: Random) -> Module | None:
    """
    Return `module` with every argument of a sample of its calls wrapped
    in redundant parentheses. `None` when no call takes an argument.
    """
    calls = [node for node in collect(module, Call) if node.args]
    if not calls:
        return None
    return rewrite(
        module,
        rng.sample(calls, k=min(len(calls), SAMPLE)),
        lambda node: node.with_changes(args=[
            arg.with_changes(value=arg.value.with_changes(lpar=[LeftParen()], rpar=[RightParen()]))
            for arg in node.args
        ]),
    )


def parsed(text: str) -> Module | None:
    """
    Return the module `text` parses to, or `None` when the parser rejects
    it or fails on a shape it does not model.
    """
    try:
        return parse_module(text)
    except (CSTLogicError, ParserSyntaxError, RecursionError):
        return None


def partition(
    body: list[BaseStatement], held: Callable[[int, BaseStatement], bool]
) -> tuple[list[BaseStatement], list[BaseStatement]]:
    """
    Split `body` into the statements `held` keeps in their seats and the
    rest, each in source order.
    """
    kept = [statement for index, statement in enumerate(body) if held(index, statement)]
    free = [statement for index, statement in enumerate(body) if not held(index, statement)]
    return kept, free


def rename_map(names: list[str], rng: Random) -> dict[str, str]:
    """
    Return a respelling for a sample of `names`, empty when none is
    renameable.
    """
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


def rewrite(module: Module, chosen: list[CSTNode], edit: Callable) -> Module:
    """
    Return `module` with `edit` applied to each of the `chosen` nodes.
    """
    picked = {id(node) for node in chosen}
    return replace(module, MatchIfTrue(lambda node: id(node) in picked), lambda node, _: edit(node))


def shuffled(module: Module, rng: Random) -> Module | None:
    """
    Return `module` with its top-level statements reordered, each keeping
    the lines it owns. A `__future__` import holds its seat ahead of the
    shuffle. `None` when fewer than two statements are free to move.
    """
    pinned, movable = partition(module.body, lambda _, statement: is_future(statement))
    if len(movable) < 2:
        return None
    rng.shuffle(movable)
    return module.with_changes(body=[*pinned, *movable])


def skipped(line: SimpleStatementLine) -> SimpleStatementLine:
    """
    Return `line` closed by a `# prose: skip` comment.
    """
    trailing = TrailingWhitespace(whitespace=SimpleWhitespace("  "), comment=Comment("# prose: skip"))
    return line.with_changes(trailing_whitespace=trailing)


def suppressed(module: Module, rng: Random) -> Module | None:
    """
    Return `module` with a `# prose: off` region wrapped around one
    top-level statement and a `# prose: skip` closing one logical line.
    `None` when it holds no top-level statement or no simple line.
    """
    lines = collect(module, SimpleStatementLine)
    if not module.body or not lines:
        return None
    module = rewrite(module, [rng.choice(lines)], skipped)
    body   = list(module.body)
    region = rng.randrange(len(body))
    body[region] = led(body[region], "# prose: off")
    if region + 1 < len(body):
        body[region + 1] = led(body[region + 1], "# prose: on", first=True)
        return module.with_changes(body=body)
    return module.with_changes(body=body, footer=[EmptyLine(comment=Comment("# prose: on")), *module.footer])


def variants(text: str, rng: Random) -> dict[str, str]:
    """
    Return each mutation's rendering of `text`, dropping the ones that do
    not apply and the ones the grammar rejects.
    """
    module = parsed(text)
    if module is None:
        return {}
    built = {}
    for mutation in MUTATIONS:
        candidate = mutation(module, rng)
        if candidate is None:
            continue
        code = candidate.code
        try:
            compile(code, mutation.__name__, "exec")
        except (SyntaxError, ValueError):
            continue
        built[mutation.__name__] = code
    return built


def widened(module: Module, rng: Random) -> Module | None:
    """
    Return `module` with a sample of its identifiers lengthened or
    shortened, shifting every column their width feeds. `None` when it
    carries no renameable name.
    """
    names   = collect(module, Name)
    renames = rename_map(sorted({node.value for node in names if not is_reserved(node.value)}), rng)
    if not renames:
        return None
    return rewrite(
        module,
        [node for node in names if node.value in renames],
        lambda node: node.with_changes(value=renames[node.value]),
    )


MUTATIONS = (commented, crlf, members, parenthesized, shuffled, suppressed, widened)


if __name__ == "__main__":

    parser = ArgumentParser(description="Write parseable mutations of a corpus, one subdirectory per mutation")
    parser.add_argument("corpus", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("budget", nargs="?", type=float, default=60)
    parser.add_argument("seed", nargs="?", default="0")
    args    = parser.parse_args()
    written = 0

    with ProcessPoolExecutor() as pool:
        futures = [
            pool.submit(mutated, path, args.corpus, args.destination, args.seed)
            for path in sorted(args.corpus.rglob("*.py"))
        ]
        try:
            for future in as_completed(futures, timeout=args.budget):
                written += future.result()
        except TimeoutError:
            pool.shutdown(cancel_futures=True)

    print(f"{written} variants written")
