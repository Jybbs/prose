#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Import each module of a corpus before and after formatting and report the
modules the rewrite breaks, each attributed to the frame it raises in and
the rules whose fixes reach it.

Usage: imports/__main__.py <binary> <python> [corpus | module]

`PROSE_IMPORTS_WIDTHS` adds widths beside the default,
`PROSE_IMPORTS_TIMEOUT` bounds one module's run in seconds,
`PROSE_IMPORTS_BAKE` names a file the break set is written to, and
`PROSE_IMPORTS_BASELINE` names one an earlier run wrote, so only a break it
does not carry fails the run.
"""

from ast                import literal_eval
from collections        import defaultdict
from concurrent.futures import ThreadPoolExecutor
from filecmp            import cmp
from functools          import partial
from json               import dumps, loads
from os                 import close, cpu_count, environ, killpg
from pathlib            import Path
from shlex              import quote
from shutil             import copy2, copytree, ignore_patterns
from signal             import SIGKILL
from subprocess         import DEVNULL, PIPE, Popen, TimeoutExpired, run
from sys                import argv
from tempfile           import mkdtemp, mkstemp

from reach import (
    bindings, covers, drops, fixes_by_file, lines_of, mapped_rows, pairing, text_of,
)
from records import Break, Outcome, Width
from report  import hunk, render

ENTRY_POINTS = {"antigravity.py", "idlelib/idle.py", "webbrowser.py"}
ENTRY_TREES  = {"idle_test", "test", "tests", "turtledemo"}
EXIT_CAP     = 4
KNOBS        = (
    "PROSE_IMPORTS_PROFILE", "PROSE_IMPORTS_PYTHON",
    "PROSE_IMPORTS_TIMEOUT", "PROSE_IMPORTS_WIDTHS"
)
MISSING = "no plain constant"
RUNNER  = Path(__file__).with_name("execute.py")


class Sweep:
    """
    One corpus, the interpreter owning it, the binary formatting it, and the
    scratch stage the trees and records land in.
    """

    def __init__(self, binary: str, python: str, target: str):
        self.binary = binary
        self.python = python

        self.stdlib, self.version = interpreter(python)
        self.corpus, self.only    = resolve(self.stdlib, target)

        self.stage     = Path(mkdtemp(prefix="prose-imports."))
        self.home      = self.stage / "home"
        self.original  = self.stage / "original"
        self.records   = self.stage / "records"
        self.tmp       = self.stage / "tmp"
        self.size      = sum(1 for _ in self.corpus.rglob("*.py"))
        self.timeout   = float(environ.get("PROSE_IMPORTS_TIMEOUT", "30"))
        self.pool      = ThreadPoolExecutor(cpu_count())
        self.rules     = [rule["slug"] for rule in loads(self.prose("rules"))]
        self.originals = {}

        copytree(self.corpus, self.original, ignore=ignore_patterns("__pycache__"))
        for directory in (self.home, self.records, self.tmp):
            directory.mkdir()

    def alone(self, brk: Break, width: int | None, label: str) -> str:
        """
        Return the rules reproducing `brk` for the same reason when every
        module its run loaded from the formatted tree is formatted under
        each one alone, joined in pipeline order.
        """
        files = brk.loaded

        def reproduces(slug: str) -> bool:
            overlay = self.overlay(label, brk.module, slug, files)
            self.configure(overlay, width)
            self.format(overlay, slug)
            after = self.execute([overlay, self.original], brk.module)
            found = divergence(self.originals[brk.module], after)
            return found is not None and found[0] == brk.reason

        verdicts = self.pool.map(reproduces, self.rules)
        return ", ".join(slug for slug, hit in zip(self.rules, verdicts) if hit)

    def attribute(
        self,
        brk       : Break,
        formatted : Path,
        covered   : dict,
        width     : int | None,
        label     : str
    ):
        """
        Fill `brk`, its frame located, with the hunk around that frame and
        the rules the format run's records attribute it to, or the rules
        reproducing it alone where no record does.
        """
        file, row = brk.frame
        pairs     = pairing(self.original / file, formatted / file)
        clauses   = []
        if row is not None:
            rows  = mapped_rows(pairs, row)
            rules = self.covering(covered, file, rows, pairs.b[row - 1])
            if rules:
                clauses.append(f"under {rules}")
        if brk.name:
            clauses += self.binding(brk, covered, brk.name)
        if not clauses and (alone := self.alone(brk, width, label)):
            clauses.append(f"reproduced by {alone} alone")
        if not clauses:
            clauses.append("no single rule reproduces it")
        brk.attribution = ", ".join(clauses)
        brk.hunk        = hunk(pairs, row)

    def binding(self, brk: Break, covered: dict, name: str) -> list[str]:
        """
        Return the clause naming where a module `brk` loaded bound `name`
        and the rules whose fixes dropped `name` from that binding, or
        rewrote it where none dropped it, empty where no loaded module binds
        it or no fix touched the binding.
        """
        for module in brk.loaded:
            rows = bindings(self.original / module).get(name)
            if rows is None:
                continue
            text  = text_of(self.original / module)
            first = lines_of(self.original / module)[rows.start - 1]
            where = f"`{name}` bound at {module}:{rows.start}"
            if dropping := self.dropping(covered, module, text, rows, name):
                return [f"{where}, dropped by {dropping}"]
            if rules := self.covering(covered, module, rows, first):
                return [f"{where}, rewritten by {rules}"]
        return []

    def candidates(self, formatted: Path) -> list[str]:
        """
        Return the modules to run, which is the one the run was pointed at,
        or every module the formatter rewrote outside the entry points.
        """
        if self.only:
            return [self.only]
        changed = []
        for path in sorted(self.original.rglob("*.py")):
            relative = path.relative_to(self.original).as_posix()
            if excluded(relative) or cmp(path, formatted / relative, shallow=False):
                continue
            changed.append(relative)
        return changed

    def configure(self, tree: Path, width: int | None):
        """
        Write the `prose.toml` pinning `width` at the root of `tree`, or
        nothing at the default width.
        """
        if width is not None:
            (tree / "prose.toml").write_text(f"code-line-length = {width}\n")

    def confirm(self, formatted: Path, brk: Break) -> bool:
        """
        Report whether a second run of both sides still breaks `brk` and the
        original agrees with its own first run.
        """
        before = self.execute([self.original], brk.module)
        after  = self.execute([formatted], brk.module)
        first  = self.originals[brk.module]
        steady = before.kind == "ok" and divergence(first, before) is None
        return steady and divergence(before, after) is not None

    def covering(
        self,
        covered : dict,
        file    : str,
        rows    : range,
        text    : str
    ) -> str:
        """
        Return the rules whose recorded fixes to `file` rewrote one of the
        original `rows` or wrote a line reading `text`, joined in pipeline
        order.
        """
        text = text.strip()
        return self.fitting(
            covered,
            file,
            lambda edits: any(covers(edit, rows, text) for edit in edits)
        )

    def dropping(
        self,
        covered : dict,
        file    : str,
        text    : str,
        rows    : range,
        name    : str
    ) -> str:
        """
        Return the rules whose recorded fixes to `file`, whose text as
        written is `text`, rewrote one of the original `rows` taking `name`
        out of the span they reach, joined in pipeline order.
        """

        def fits(edits: list[dict]) -> bool:
            reached = any(covers(edit, rows, "") for edit in edits)
            return reached and drops(text, edits, name)

        return self.fitting(covered, file, fits)

    def execute(self, trees: list[Path], relative: str) -> Outcome:
        """
        Run the module at `relative` from the first of `trees` carrying it
        in a fresh interpreter and return what it left behind.
        """
        descriptor, record = mkstemp(dir=self.records)
        close(descriptor)
        command = [
            self.python, "-I", "-B",
            str(RUNNER),
            record, relative,
            *map(str, trees)
        ]
        env = {"HOME": str(self.home), "PATH": environ["PATH"], "TMPDIR": str(self.tmp)}
        child = Popen(
            command,
            cwd               = self.tmp,
            env               = env,
            start_new_session = True,
            stderr            = PIPE,
            stdin             = DEVNULL,
            stdout            = DEVNULL
        )
        try:
            _, err = child.communicate(timeout=self.timeout)
        except TimeoutExpired:
            killpg(child.pid, SIGKILL)
            child.stderr.close()
            child.wait()
            return Outcome("timeout", f"times out after {self.timeout:g}s")
        if left := Path(record).read_text(encoding="utf-8"):
            try:
                return outcome(literal_eval(left))
            except (SyntaxError, ValueError):
                return Outcome("unmeasured", "leaves an unreadable record")
        if child.returncode < 0:
            return Outcome("raised", f"dies on signal {-child.returncode}")
        if child.returncode:
            printed = last_line(err.decode(errors="replace"))
            return Outcome("raised", f"exits {child.returncode} printing {printed}")
        return Outcome("unmeasured", "leaves no record")

    def fitting(self, covered: dict, file: str, fits) -> str:
        """
        Return the rules whose recorded fixes to `file` satisfy `fits`,
        joined in pipeline order.
        """
        hit = {slug for slug, edits in covered.get(file, []) if fits(edits)}
        return ", ".join(sorted(hit, key=self.rules.index))

    def format(self, target: Path, select: str | None = None) -> tuple[list[dict], str]:
        """
        Format `target` in place under every rule, or under `select` alone,
        and return the run's diagnostic records with its stderr.
        """
        flags = ["--no-cache", "--output-format", "json"]
        if select:
            flags += ["--select", select]
        done = run(
            [self.binary, "format", *flags, str(target)],
            cwd    = self.stage,
            stderr = PIPE,
            stdout = PIPE,
            text   = True
        )
        if done.returncode > EXIT_CAP:
            raise SystemExit(
                f"format exited {done.returncode} on {target}: {last_line(done.stderr)}"
            )
        return [loads(line) for line in done.stdout.splitlines()], done.stderr

    def locate(self, brk: Break, formatted: Path) -> tuple[str, int | None]:
        """
        Return the file and row `brk` names, taking the deepest traceback
        frame under `formatted`, otherwise the row binding the name it
        hinges on, in the formatted module or mapped forward from the
        original, and the module alone where neither exists.
        """
        under = [
            (Path(file).relative_to(formatted).as_posix(), line)
            for file, line in brk.formatted.frames
            if file.startswith(f"{formatted}/")
        ]
        if under:
            return under[-1]
        if not brk.name:
            return brk.module, None
        if rows := bindings(formatted / brk.module).get(brk.name):
            return brk.module, rows.start
        if rows := bindings(self.original / brk.module).get(brk.name):
            pairs  = pairing(self.original / brk.module, formatted / brk.module)
            landed = mapped_rows(pairs, rows.start, back=False)
            return brk.module, min(landed.start, len(pairs.b)) if landed else None
        return brk.module, None

    def outcomes(self, tree: Path, modules: list[str]) -> dict[str, Outcome]:
        """
        Return what each of `modules` leaves behind when run from `tree`,
        the runs sharing the worker pool.
        """
        return dict(
            zip(
                modules,
                self.pool.map(lambda module: self.execute([tree], module), modules)
            )
        )

    def overlay(
        self,
        label  : str,
        module : str,
        slug   : str,
        files  : tuple[str, ...]
    ) -> Path:
        """
        Return a tree holding the original of each of `files`, every
        other entry of their directories linked to the original, ready
        to be formatted under `slug` alone ahead of the original tree on
        `sys.path`.
        """
        overlay = self.stage / "alone" / label / module.replace("/", "+") / slug
        for file in files:
            target = overlay / file
            target.parent.mkdir(exist_ok=True, parents=True)
            copy2(self.original / file, target)
        for parent in {(overlay / file).parent for file in files}:
            for entry in (self.original / parent.relative_to(overlay)).iterdir():
                target = parent / entry.name
                if entry.name != "__pycache__" and not target.exists():
                    target.symlink_to(entry)
        return overlay

    def prose(self, *arguments: str) -> bytes:
        """
        Return the stdout of the binary run with `arguments`.
        """
        command = [self.binary, *arguments, "--output-format", "json"]
        return run(command, check=True, stdout=PIPE).stdout

    def reproduction(self, module: str, width: int | None) -> str:
        """
        Return the command running `module` alone at `width`.
        """
        knobs = [f"{knob}={quote(environ[knob])}" for knob in KNOBS if knob in environ]
        if width is not None and "PROSE_IMPORTS_WIDTHS" not in environ:
            knobs.append(f"PROSE_IMPORTS_WIDTHS={width}")
        target = quote(str(self.corpus / module))
        return " ".join([*knobs, "mise", "run", "imports", target])

    def sweep(self, width: int | None) -> Width:
        """
        Format a copy of the corpus at `width`, run every module the
        formatter rewrote from both trees, confirm each break, and attribute
        it.
        """
        label     = "default" if width is None else str(width)
        formatted = self.stage / f"formatted-{label}"
        copytree(self.corpus, formatted, ignore=ignore_patterns("__pycache__"))
        self.configure(formatted, width)
        records, log = self.format(formatted)
        (self.stage / f"format-{label}.log").write_text(log, encoding="utf-8")
        covered = fixes_by_file(records, formatted)
        modules = self.candidates(formatted)
        fresh   = [module for module in modules if module not in self.originals]
        self.originals.update(self.outcomes(self.original, fresh))
        after    = self.outcomes(formatted, modules)
        found    = Width(label, len(modules))
        suspects = []
        for module in modules:
            before, latest = self.originals[module], after[module]
            if "unmeasured" in (before.kind, latest.kind):
                found.unmeasured.append(module)
            elif before.kind != "ok":
                found.uncomparable += 1
            else:
                found.comparable += 1
                if differs := divergence(before, latest):
                    suspects.append(Break(module, *differs, latest))
        verdicts = self.pool.map(lambda brk: self.confirm(formatted, brk), suspects)
        for brk, confirmed in zip(suspects, verdicts):
            if confirmed:
                found.breaks.append(brk)
            else:
                found.flaky.append(brk.module)
        alike = defaultdict(list)
        for brk in found.breaks:
            brk.frame = self.locate(brk, formatted)
            alike[(brk.frame, brk.reason)].append(brk)
        for members in alike.values():
            self.attribute(members[0], formatted, covered, width, label)
            for brk in members[1:]:
                brk.attribution, brk.hunk = members[0].attribution, members[0].hunk
        return found


def divergence(original: Outcome, formatted: Outcome) -> tuple[str, str | None] | None:
    """
    Return why `formatted` counts as broken beside `original` and the name
    it hinges on, or `None` where both bind the same namespace.
    """
    if formatted.kind != "ok":
        return formatted.error, formatted.name
    bound, rebound = set(original.names), set(formatted.names)
    if unbound := sorted(bound - rebound):
        return f"leaves `{unbound[0]}` unbound", unbound[0]
    if extra := sorted(rebound - bound):
        return f"binds `{extra[0]}` the original does not", extra[0]
    before, after = dict(original.constants), dict(formatted.constants)
    for name in sorted(before | after):
        if before.get(name) != after.get(name):
            was, now = before.get(name, MISSING), after.get(name, MISSING)
            return f"binds `{name}` to {now} where the original binds {was}", name
    return None


def excluded(relative: str) -> bool:
    """
    Report whether `relative` is an entry point rather than a library
    module.
    """
    parts = relative.split("/")
    return (
        parts[-1]   == "__main__.py"
        or relative in ENTRY_POINTS
        or not ENTRY_TREES.isdisjoint(parts[:-1])
    )


def interpreter(python: str) -> tuple[Path, str]:
    """
    Return the standard library `python` owns and its version.
    """
    probe = (
        "import sys, sysconfig\n"
        "print(sysconfig.get_paths()['stdlib'])\n"
        "print(sys.version.split()[0])"
    )
    try:
        done = run([python, "-I", "-c", probe], check=True, stdout=PIPE, text=True)
    except OSError as error:
        raise SystemExit(f"{python or 'python'} does not run: {error}") from None
    stdlib, version = done.stdout.splitlines()
    return Path(stdlib).resolve(), version


def last_line(text: str) -> str:
    """
    Return the last line `text` carries, or an empty string.
    """
    return text.strip().rsplit("\n", 1)[-1]


def outcome(left: dict) -> Outcome:
    """
    Return the outcome a runner's record describes.
    """
    loaded = tuple(left["loaded"])
    if left["outcome"] == "raised":
        return Outcome(
            "raised",
            f"raises {left['error']}",
            tuple(left["frames"]),
            loaded,
            left["name"]
        )
    names     = tuple(left["names"])
    constants = tuple(sorted(left["constants"].items()))
    return Outcome("ok", constants=constants, loaded=loaded, names=names)


def resolve(stdlib: Path, target: str) -> tuple[Path, str | None]:
    """
    Return the corpus `target` names and the one module it narrows the run
    to, refusing a corpus the interpreter owning `stdlib` does not.
    """
    if not target:
        return stdlib, None
    path = Path(target).resolve()
    if path.is_dir():
        if path != stdlib:
            raise SystemExit(
                f"{path} is not the interpreter's standard library, {stdlib}"
            )
        return stdlib, None
    if path.suffix != ".py" or not path.is_file():
        raise SystemExit(f"{target} is neither a directory nor a module")
    if not path.is_relative_to(stdlib):
        raise SystemExit(
            f"{path} is outside the interpreter's standard library, {stdlib}"
        )
    return stdlib, path.relative_to(stdlib).as_posix()


if __name__ == "__main__":

    binary, python, *rest = argv[1:]
    sweep                 = Sweep(binary, python, next(iter(rest), ""))
    widths   = [None, *map(int, environ.get("PROSE_IMPORTS_WIDTHS", "").split())]
    baseline = {}
    if named := environ.get("PROSE_IMPORTS_BASELINE"):
        baseline = loads(Path(named).read_text(encoding="utf-8"))

    print(
        f"corpus      {sweep.corpus} ({sweep.size} files)\n"
        f"binary      {sweep.binary}\n"
        f"interpreter {sweep.python} ({sweep.version})\n"
        f"stage       {sweep.stage}",
        flush = True
    )

    reports = []
    for width in widths:
        found     = sweep.sweep(width)
        reproduce = partial(sweep.reproduction, width=width)
        broken    = {brk.module for brk in found.breaks}
        held      = baseline.get(found.label, {})
        known     = {tuple(key) for key in held.get("frames", [])}
        carried   = {brk.module for brk in found.breaks if (
            brk.frame[0],
            brk.reason
        ) in known}
        grown = carried - set(held.get("modules", []))
        print(
            f"\nwidth {found.label}\n{render(reproduce, found, carried)}",
            flush = True
        )
        if grown:
            print(f"  grown        {len(grown):>5}", flush=True)
        reports.append((found, broken, carried))

    print(f"\neach tree survives under {sweep.stage}")
    if any(found.unmeasured for found, _, _ in reports):
        raise SystemExit(
            "a run left modules unmeasured, so the uncomparable count cannot be named"
        )
    if baked := environ.get("PROSE_IMPORTS_BAKE"):
        breaks = {
            found.label: {
                "frames"  : sorted(
                    {(brk.frame[0], brk.reason) for brk in found.breaks}
                ),
                "modules" : sorted(broken)
            }
            for found, broken, _ in reports
        }
        Path(baked).write_text(dumps(breaks, indent=2) + "\n", encoding="utf-8")
        print(f"break set baked into {baked}")
        raise SystemExit(0)
    raise SystemExit(
        1 if any(broken - carried for _, broken, carried in reports) else 0
    )
