#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["tabulate==0.10.0"]
# ///
"""
Render the delta between a stage's base and head cycles at each width.

Usage: delta.py [--markdown] <stage> <width>...

Reads `<stage>/.git/base-<width>.ndjson` and `head-<width>.ndjson`, each
carrying a run's summary record and one `{code, filename}` record per
fix, and renders per width the rules whose firing count moved, sorted
by the size of the move, the files each rule newly fires on or no longer
fires on, and git's diffstat between the two tags. `--markdown` renders
the shape a step summary takes, appending to `$GITHUB_STEP_SUMMARY` when
it is set and printing otherwise.
"""

from argparse    import ArgumentParser
from collections import defaultdict
from json        import loads
from os          import environ
from pathlib     import Path
from subprocess  import run

from tabulate import tabulate

SHOWN = 3


class Cycle:
    """
    One tagged cycle's summary record and the files each rule fired on.
    """

    def __init__(self, path: Path):
        self.fired: dict[str, set[str]] = defaultdict(set)
        self.summary: dict = {}
        for line in path.read_text(encoding="utf-8").splitlines():
            record = loads(line)
            if record.get("kind") == "summary":
                self.summary = record
            else:
                self.fired[record["code"]].add(record["filename"])

    @property
    def counts(self) -> dict[str, int]:
        return self.summary.get("rules_fired", {})

    @property
    def unstable(self) -> int:
        return len(self.summary.get("unstable", []))


class Report:
    """
    Render one width's delta as terminal text or as Markdown.
    """

    def __init__(self, stage: Path, width: str, markdown: bool):
        self.base     = Cycle(stage / ".git" / f"base-{width}.ndjson")
        self.head     = Cycle(stage / ".git" / f"head-{width}.ndjson")
        self.markdown = markdown
        self.stage    = stage
        self.width    = width

    def code(self, text: str) -> str:
        """
        Render `text` as inline code in Markdown, bare otherwise.
        """
        return f"`{text}`" if self.markdown else text

    def counts(self) -> str:
        """
        Table the rules whose firing count moved, largest move first.
        """
        rows = sorted(
            (
                (slug, before, after, after - before)
                for slug in self.base.counts.keys() | self.head.counts.keys()
                if (before := self.base.counts.get(slug, 0)) != (after := self.head.counts.get(slug, 0))
            ),
            key = lambda row: (-abs(row[3]), row[0]),
        )
        if not rows:
            return self.lines("every rule fired the same number of times")
        cells = [(self.code(slug), before, after, f"{delta:+}") for slug, before, after, delta in rows]
        if self.markdown:
            return tabulate(cells, headers=["Rule", "Base", "Head", "Delta"], tablefmt="github") + "\n\n"
        return self.lines(tabulate(cells, tablefmt="plain"))

    def diffstat(self) -> str:
        """
        Git's own diffstat between the two tags, capped at five files.
        """
        stat = run(
            ["git", "-C", self.stage, "diff", "--stat-count=5", f"base-{self.width}", f"head-{self.width}"],
            capture_output = True,
            check          = True,
            text           = True,
        ).stdout
        text = "\n".join(line.strip() for line in stat.splitlines()) or "no file differs"
        if self.markdown:
            return f"```\n{text}\n```\n\n"
        return self.lines(text)

    def lines(self, text: str) -> str:
        """
        Render `text` as a bullet list in Markdown, indented lines otherwise.
        """
        marker = "- " if self.markdown else "  "
        return "".join(f"{marker}{line}\n" for line in text.splitlines()) + ("\n" if self.markdown else "")

    def movements(self) -> str:
        """
        Name the files each rule newly fires on or no longer fires on.
        """
        moves = sorted(
            (
                (-len(files), slug, verb, files)
                for slug in self.base.fired.keys() | self.head.fired.keys()
                for verb, files in [
                    ("newly fires", self.head.fired[slug] - self.base.fired[slug]),
                    ("no longer fires", self.base.fired[slug] - self.head.fired[slug]),
                ]
                if files
            )
        )
        if not moves:
            return self.lines("every rule fires on the same files")
        return self.lines("\n".join(
            f"{self.code(slug)} {verb} on {len(files)} file{'s' if len(files) > 1 else ''} ({self.named(files)})"
            for _, slug, verb, files in moves
        ))

    def named(self, files: set[str]) -> str:
        """
        List the first `SHOWN` of `files` and count the rest.
        """
        shown = sorted(files)
        names = ", ".join(self.code(name) for name in shown[:SHOWN])
        return names if len(shown) <= SHOWN else f"{names}, and {len(shown) - SHOWN} more"

    def render(self) -> str:
        """
        Render the width's heading, count table, movements, stability, and diffstat.
        """
        heading = f"### Width {self.width}\n\n" if self.markdown else f"width {self.width}\n"
        return heading + self.counts() + self.movements() + self.stability() + self.diffstat()

    def stability(self) -> str:
        """
        Name a side whose summary carries unstable entries at this width.
        """
        notes = [
            f"{side} is unstable on {cycle.unstable} of the files at this width"
            for side, cycle in [("base", self.base), ("head", self.head)]
            if cycle.unstable
        ]
        return self.lines("\n".join(notes)) if notes else ""


def heading(stage: Path) -> str:
    """
    Name the baseline the stage was baked from and the head it was read against.
    """
    baseline = (stage / ".git" / "baseline").read_text(encoding="utf-8").strip()
    head     = run(["git", "describe", "--always", "--dirty"], capture_output=True, check=True, text=True)
    return f"## 🦋 Delta\n\nBaseline `{baseline}` against head `{head.stdout.strip()}`.\n\n"


if __name__ == "__main__":

    parser = ArgumentParser(description="Render the delta between a stage's base and head cycles")
    parser.add_argument("--markdown", action="store_true")
    parser.add_argument("stage", type=Path)
    parser.add_argument("widths", nargs="+")
    args = parser.parse_args()
    text = "".join(Report(args.stage, width, args.markdown).render() for width in args.widths)

    if args.markdown:
        text = heading(args.stage) + text

    if args.markdown and (path := environ.get("GITHUB_STEP_SUMMARY")):
        with open(path, "a", encoding="utf-8") as f:
            f.write(text)
    else:
        print(text, end="")
