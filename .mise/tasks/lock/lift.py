#!/usr/bin/env -S uv run --script
# MISE alias="lift"
# MISE description="List, try, and write every outdated pin"
# /// script
# requires-python = ">=3.11"
# dependencies = ["packaging==26.3", "tomlkit==0.15.1"]
# ///
"""
Lists every pin behind its newest release as one Markdown table, then tries
the lifts in a scratch clone under `--try` and writes them under `--write`.

A pin is read from each home it is written in. Pins sharing a name, which
covers every set `repo:audit` holds equal, take one row and move together,
and so do the `ruff_*` crates with the release `[package.metadata.ruff]`
names. The reports of mise, Cargo, uv, and bun give the newest release of
the pins they cover, and a registry gives it for every other pin.

`--try` lifts every pin in a scratch clone carrying the checkout's
uncommitted and untracked files and runs `mise ci` there, logging each
attempt under `.cache/lift/`. Where that attempt fails, it retries with each
major lift held back in turn and stops at the first that passes. `--write`
writes each lift into its homes, locks and installs each tool a lifted mise
pin names, moves each lifted package in its manager's lockfile, and runs
`mise relock`, taking the holds of the attempt that passed when `--try` is
given too. `--hold <pin>` leaves a pin where it stands, and a name the table
omits exits 2 before any trial or write.
"""

from argparse           import ArgumentParser
from collections        import defaultdict
from collections.abc    import Iterator
from concurrent.futures import ThreadPoolExecutor
from dataclasses        import dataclass, replace
from datetime           import UTC, datetime
from functools          import partial, reduce
from gzip               import decompress
from io                 import BytesIO
from json               import loads as from_json
from operator           import getitem
from os                 import environ, pathsep
from packaging.requirements import Requirement
from packaging.utils        import canonicalize_name
from packaging.version      import InvalidVersion, Version
from pathlib                import Path
from re             import escape, finditer, search, sub
from shlex          import join
from shutil         import copy2, rmtree
from subprocess     import PIPE, STDOUT, run
from sys            import stderr
from tarfile        import open as open_tar
from tempfile       import TemporaryDirectory
from time           import sleep
from tomlkit        import parse
from tomllib        import loads as from_toml
from typing         import TextIO
from urllib.parse   import quote
from urllib.request import Request, urlopen

AGENT        = "prose lock:lift (https://github.com/Jybbs/prose)"
CONFIG       = ".mise/config.toml"
COVERED      = {"bun", "cargo", "mise", "uv"}
CRATES       = "https://crates.io/api/v1/crates"
DEPENDENCIES = [
    ("workspace", "dependencies"),
    ("build-dependencies",),
    ("dependencies",),
    ("dev-dependencies",)
]
ECOSYSTEMS = {
    "action" : "actions",
    "bun"    : "npm",
    "cargo"  : "rust",
    "npm"    : "npm",
    "pypi"   : "pip",
    "uv"     : "pip"
}
INPUTS = {"maturin-version": ("maturin", ""), "wranglerVersion": ("wrangler", "npm")}
LOGS   = Path(".cache/lift")
PYPI   = "https://pypi.org/pypi"
RUFF   = "ruff"
SCRIPT_BLOCK = r"(?m)^# /// script$\s(?P<content>(?:^#(?:| .*)$\s)+)^# ///$"
USES         = (
    r"uses\s*:\s*(?:&[\w-]+\s+)?(?P<repo>[\w.-]+/[\w.-]+)(?:/[\w./-]*)?"
    r"@(?P<written>[0-9a-f]{40})\b"
)


@dataclass(frozen=True)
class Home:
    """
    Holds one place a pin is written, as a file beside the keys `tomlkit`
    reaches the pin by in TOML, or a pattern whose every match holds the pin
    in any other file. `parts` counts the components of the version the home
    writes, where zero writes them all.
    """

    path    : str
    keys    : tuple[str, ...] = ()
    parts   : int             = 0
    pattern : str             = ""

    def rewrite(self, new: str, old: str, text: str) -> str:
        """
        Returns `text` with the pin this home holds moved from `old` to
        `new`, keeping any operator ahead of the version and every other
        byte as it stood.
        """
        if not self.keys:
            bounded = rf"(?<![\d.]){escape(old)}(?![\d.])"
            return sub(self.pattern, lambda match: sub(bounded, new, match[0]), text)

        document     = parse(text)
        *tables, key = self.keys
        table        = reduce(getitem, tables, document)
        if isinstance(table[key], dict):
            table, key = table[key], "version"
        table[key] = table[key].removesuffix(old) + new
        return document.as_string()

    def shown(self, version: str) -> str:
        """
        Returns `version` as this home writes it, without build metadata and
        cut to its first `parts` components where `parts` is set.
        """
        plain = version.partition("+")[0]
        return ".".join(plain.split(".")[: self.parts]) if self.parts else plain


@dataclass(frozen=True)
class Pin:
    """
    Holds one pinned version as its home writes it, the row it is listed
    under, the package its source knows it by, and that source. A source is
    a manager whose report covers the pin (`bun`, `cargo`, `mise`, `uv`), a
    registry read for the pin alone (`action`, `npm`, `pypi`), or empty for
    a home that follows the rest of its row. A pin the lockfile alone holds
    carries no home.
    """

    home    : Home | None
    name    : str
    package : str
    source  : str
    written : str

    @property
    def covered(self) -> bool:
        """
        Returns whether a manager's report covers this pin.
        """
        return self.source in COVERED

    @property
    def ecosystem(self) -> str | None:
        """
        Returns the ecosystem a Dependabot alert names this pin's package
        under, or `None` for a source Dependabot does not read.
        """
        return ECOSYSTEMS.get(self.source)

    @property
    def script(self) -> str | None:
        """
        Returns the task script whose PEP 723 block holds this pin, or
        `None` where another home or the lockfile alone holds it.
        """
        return self.home.path if self.home and self.home.path.endswith(".py") else None

    @classmethod
    def from_requirement(cls, path: str, requirement: str, source: str) -> "Pin | None":
        """
        Returns the pin a Python `requirement` written in the file at `path`
        holds, or `None` where no `==` specifier pins it to one release.
        """
        parsed  = Requirement(requirement)
        version = next(
            (spec.version for spec in parsed.specifier if spec.operator == "=="),
            None
        )
        if version is None:
            return None
        return cls(
            home    = Home(path, pattern=rf'"{escape(parsed.name)}(?![\w.-])[^"]*"'),
            name    = canonicalize_name(parsed.name),
            package = parsed.name,
            source  = source,
            written = version
        )

    def released(self, version: str | None = None) -> "Release":
        """
        Returns the release of this pin's package at `version`, or its
        newest release where no version is given, beside when its registry
        says it was published. A tool's time is the `created_at` mise lists
        for it, a crate's comes from crates.io, an npm package's from its
        registry's `time` map, and a Python package's from the earliest file
        PyPI lists.
        """
        if self.source == "mise":
            listed  = from_json(output("mise", "ls-remote", "--json", self.package))
            created = {each["version"]: each.get("created_at") for each in listed}
            return Release(version, stamp(created.get(version)))
        if self.source == "cargo":
            sleep(1)
            document = fetched(f"{CRATES}/{self.package}/{quote(version)}")
            return Release(version, stamp(document["version"]["created_at"]))
        if self.source in {"bun", "npm"}:
            document = packument(self.package)
            version  = version or document["dist-tags"]["latest"]
            return Release(version, stamp(document["time"].get(version)))
        document = fetched(
            url = "/".join(filter(None, [PYPI, self.package, version, "json"]))
        )
        files = [file["upload_time_iso_8601"] for file in document["urls"]]
        return Release(document["info"]["version"], stamp(min(files, default=None)))


@dataclass(frozen=True)
class Release:
    """
    Holds a release's version beside the time it was published, which a
    registry may leave unreported.
    """

    version   : str
    published : datetime | None = None


@dataclass(frozen=True)
class Reports:
    """
    Holds the newest release each manager's report gives for the packages it
    covers, keyed by the manager, the package, and the version it resolves
    today. A package a report omits stands at its newest release.
    """

    releases: dict[tuple[str, str, str], str]

    def newest(self, pin: Pin) -> str:
        """
        Returns the newest release the report covering `pin` gives, which is
        its written version where the report omits it.
        """
        reported = (pin.source, canonicalize_name(pin.package), pin.written)
        return self.releases.get(reported, pin.written)

    @classmethod
    def read(cls, scripts: list[str]) -> "Reports":
        """
        Returns the reports `bun outdated`, `cargo update --dry-run`, `mise
        outdated`, and `uv tree --outdated` print, the last over the project
        and each of `scripts`, run side by side.
        """
        with ThreadPoolExecutor() as pool:
            bun   = pool.submit(output, "bun", "outdated", cwd="site")
            cargo = pool.submit(output, "cargo", "update", "--dry-run", "--verbose")
            mise  = pool.submit(output, "mise", "outdated", "--bump", "--json")
            uv    = [
                pool.submit(output, "uv", "tree", "--depth", "1", "--outdated", *target)
                for target in [
                    ["--all-groups", "--frozen", "--project", "crate"],
                    *(["--script", script] for script in scripts)
                ]
            ]

        bun_row = (
            r"(?m)^\|\s*(\S+)(?: \(\w+\))?\s*\|"
            r"\s*(\S+)\s*\|\s*\S+\s*\|\s*(\d\S*)\s*\|$"
        )
        cargo_line = (
            r"(?m)^\s*(?:Updating|Unchanged) (\S+) v([^\s+]+)\S*"
            r"(?: -> v(\S+))?(?: \(available: v(\S+)\))?$"
        )
        uv_line = r"(?m)^[\s│├└─]*(\S+) v(\S+)\b.*\(latest: v(\S+)\)$"
        found   = [
            *(("bun", *match.groups()) for match in finditer(bun_row, bun.result())),
            *(
                ("cargo", name, old, available or new)
                for name, old, new, available in (
                    match.groups() for match in finditer(cargo_line, cargo.result())
                )
            ),
            *(
                ("mise", key, tool["requested"], tool["latest"])
                for key, tool in from_json(mise.result()).items()
            ),
            *(
                ("uv", *match.groups())
                for each in uv
                for match in finditer(uv_line, each.result())
            )
        ]
        return cls(
            {
                (source, canonicalize_name(name), current): newest
                for source, name, current, newest in found
                if newest and newest != current
            }
        )


@dataclass(frozen=True)
class Lift:
    """
    Holds one row of the table, meaning the pins it moves, the version they
    stand at and their newest release, when that release was published,
    the open Dependabot alerts the lift clears, and the text each written
    version becomes.
    """

    current   : str
    name      : str
    newest    : str
    pins      : tuple[Pin, ...]
    published : datetime | None
    targets   : dict[str, str]
    alerts    : tuple[int, ...] = ()

    @property
    def days(self) -> str:
        """
        Returns how many whole days ago the newest release was published, or
        nothing where its registry reports no time.
        """
        return str((datetime.now(UTC) - self.published).days) if self.published else ""

    @property
    def edits(self) -> set[tuple[Home, str, str]]:
        """
        Returns each home this row moves beside the text its written version
        becomes and that version.
        """
        return {
            (pin.home, self.targets[pin.written], pin.written)
            for pin in self.pins
            if pin.home
        }

    @property
    def major(self) -> bool:
        """
        Returns whether the lift changes the left-most non-zero part of
        the version, so `0.33.1` to `0.34.0` is major and `2026.7.0` to
        `2026.9.0` is not.
        """
        current, newest = ordered(self.current).release, ordered(self.newest).release
        leading         = next(
            (index for index, part in enumerate(current) if part),
            len(current) - 1
        )
        return current[: leading + 1] != newest[: leading + 1]

    @classmethod
    def action(cls, pins: list[Pin], repo: str) -> "Lift | None":
        """
        Returns the row moving every home of the `repo` action from the
        commit it pins to the commit its latest release tags, each shown as
        the most specific tag naming it.
        """
        commit  = agreed(repo, pins)
        tags    = tagged(repo)
        latest  = release(repo)
        current = max(
            (tag for tag, sha in tags.items() if sha == commit),
            default = commit[:12],
            key     = lambda tag: (ordered(tag), tag.count("."))
        )
        if tags[latest.version] == commit:
            return None

        return cls(
            current   = current,
            name      = repo,
            newest    = latest.version,
            pins      = tuple(pins),
            published = latest.published,
            targets   = {commit: tags[latest.version]}
        )

    def cells(self) -> list[str]:
        """
        Returns the cells of this row in the table's column order.
        """
        return [
            f"`{self.name}`",
            f"`{self.current}`",
            f"`{self.newest}`",
            "yes" if self.major else "no",
            self.days,
            ", ".join(f"#{number}" for number in self.alerts)
        ]

    def clearing(self, alerts: list[dict]) -> "Lift":
        """
        Returns this row carrying each of `alerts` it clears, meaning an
        alert on one of its packages whose first patched version the newest
        release reaches.
        """
        packages = {
            (pin.ecosystem, canonicalize_name(pin.package))
            for pin in self.pins
            if pin.ecosystem
        }
        cleared = tuple(
            alert["number"] for alert in alerts
            if (
                alert["dependency"]["package"]["ecosystem"],
                canonicalize_name(alert["dependency"]["package"]["name"])
            ) in packages
            if (patched := alert["security_vulnerability"]["first_patched_version"])
            if ordered(patched["identifier"]) <= ordered(self.newest)
        )
        return replace(self, alerts=cleared)

    @classmethod
    def of(cls, name: str, pins: list[Pin], reports: Reports) -> "Lift | None":
        """
        Returns the row lifting `pins` to their newest release, or `None`
        where they already stand at it. The pins carrying a source of their
        own agree on the version they stand at, and where a report covers
        any of them, the reports alone give the newest release.
        """
        if any(pin.source == "action" for pin in pins):
            return cls.action(pins, name)
        if name == RUFF:
            return cls.ruff(pins, reports)

        sourced = [pin for pin in pins if pin.source]
        current = agreed(name, sourced)
        covered = [pin for pin in sourced if pin.covered]
        found   = [(pin, Release(reports.newest(pin))) for pin in covered] or [
            (pin, pin.released()) for pin in sourced
        ]
        origin, newest = max(found, key=lambda each: ordered(each[1].version))
        if ordered(newest.version) <= ordered(current):
            return None

        if covered:
            newest = origin.released(newest.version)
        return cls(
            current   = current,
            name      = name,
            newest    = newest.version,
            pins      = tuple(pins),
            published = newest.published,
            targets   = {
                pin.written: pin.home.shown(newest.version) for pin in pins if pin.home
            }
        )

    @classmethod
    def ruff(cls, pins: list[Pin], reports: Reports) -> "Lift | None":
        """
        Returns the row moving the `ruff_*` crates to their newest release
        beside the ruff release `[package.metadata.ruff]` names, which is
        the tag on the commit the newest crate was published from.
        """
        crates  = [pin for pin in pins if pin.source]
        crate   = agreed(RUFF, crates)
        newest  = max((reports.newest(pin) for pin in crates), key=ordered)
        current = agreed(RUFF, [pin for pin in pins if not pin.source])
        if newest == crate:
            return None

        commit = published_commit(crates[0].package, newest)
        named  = [tag for tag, sha in tagged("astral-sh/ruff").items() if sha == commit]
        if not named:
            raise SystemExit(
                f"no ruff release tags {commit}, the commit {newest} came from"
            )
        tag = max(named, key=ordered)
        return cls(
            current   = current,
            name      = RUFF,
            newest    = tag,
            pins      = tuple(pins),
            published = crates[0].released(newest).published,
            targets   = {crate: newest, current: tag}
        )


def alerts() -> list[dict]:
    """
    Returns every open Dependabot alert on the repository `gh` resolves,
    warning and returning none where `gh` cannot read them.
    """
    read = run(
        [
            "gh", "api", "--paginate",
            "--slurp", "repos/{owner}/{repo}/dependabot/alerts?state=open&per_page=100"
        ],
        capture_output = True,
        text           = True
    )
    if read.returncode:
        print(f"Dependabot alerts were not read: {read.stderr.strip()}", file=stderr)
        return []
    return [alert for page in from_json(read.stdout) for alert in page]


def attempt(holds: set[str], log: Path) -> bool:
    """
    Returns whether `mise ci` passes in a scratch clone of the checkout
    once every lift but `holds` is written there, logging each command the
    attempt runs and all it prints to `log`. The virtualenv the clone's
    mise config declares is rebuilt between the write and `mise ci` on the
    Python the clone then pins, since the write runs under the one it pinned
    before.
    """
    root = Path.cwd()
    with TemporaryDirectory(prefix="prose-lift.") as scratch, log.open("w") as out:
        clone = Path(scratch, "clone")
        output("git", "clone", "--quiet", "--shared", str(root), str(clone))
        carry(clone, root)

        venv = nested(("env", "_", "python", "venv", "path"), toml(CONFIG))
        step = partial(logged, clone, environment=isolated(clone, root), out=out)
        if not step(["mise", "run", "lock:lift", "--write", *flagged("--hold", holds)]):
            return False
        rmtree(clone / venv, ignore_errors=True)
        return step(["mise", "run", "ci"])


def bare(tool: str) -> str:
    """
    Returns the name of the mise `tool` without its backend and owner, so
    `aqua:mitsuhiko/insta` reads as `insta`.
    """
    return tool.rpartition(":")[2].rpartition("/")[2]


def carry(clone: Path, root: Path):
    """
    Copies every file the checkout at `root` changes from its `HEAD`, along
    with every untracked file git does not ignore, into `clone`, and removes
    there each tracked file the checkout deleted.
    """
    changed = output("git", "diff", "--name-only", "--no-renames", "-z", "HEAD")
    added   = output("git", "ls-files", "--exclude-standard", "--others", "-z")
    for path in filter(None, {*changed.split("\0"), *added.split("\0")}):
        target = clone / path
        if (root / path).exists():
            target.parent.mkdir(exist_ok=True, parents=True)
            copy2(root / path, target, follow_symlinks=False)
        else:
            target.unlink(missing_ok=True)


def downloaded(url: str) -> bytes:
    """
    Returns the body at `url`, requested under the user agent crates.io asks
    every API client to send and decompressed where the server gzipped it.
    """
    request = Request(url, headers={"Accept-Encoding": "gzip", "User-Agent": AGENT})
    with urlopen(request, timeout=120) as response:
        body    = response.read()
        encoded = response.headers.get("Content-Encoding") == "gzip"
    return decompress(body) if encoded else body


def fetched(url: str) -> dict:
    """
    Returns the JSON document at `url`.
    """
    return from_json(downloaded(url))


def flagged(option: str, values: set[str]) -> list[str]:
    """
    Returns `option` ahead of each of `values` in sorted order, the way a
    command takes a flag once per value.
    """
    return [each for value in sorted(values) for each in (option, value)]


def isolated(clone: Path, root: Path) -> dict[str, str]:
    """
    Returns the environment a command in `clone` runs under, which is this
    task's own without the variables the checkout's mise config and its
    task runner set and without each `PATH` entry inside the checkout, so
    mise sets the clone's own in their place. The clone's config is trusted
    for those commands alone, and `gh` reads the repository the checkout's
    remote names.
    """
    dropped = {*toml(CONFIG).get("env", {}), "MISE_ORIGINAL_CWD", "VIRTUAL_ENV"} - {"_"}
    kept    = {
        name: value for name, value in environ.items()
        if name not in dropped
        if not name.startswith(("MISE_CONFIG_", "MISE_PROJECT_", "MISE_TASK_"))
    }
    paths = [
        entry for entry in environ.get("PATH", "").split(pathsep)
        if not Path(entry).resolve().is_relative_to(root.resolve())
    ]
    named = run(
        ["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"],
        capture_output = True,
        text           = True
    )
    return {
        **kept,
        **({"GH_REPO": named.stdout.strip()} if named.returncode == 0 else {}),
        "MISE_TRUSTED_CONFIG_PATHS": str(clone),
        "PATH": pathsep.join(paths)
    }


def logged(
    clone       : Path,
    command     : list[str],
    environment : dict[str, str],
    out         : TextIO
) -> bool:
    """
    Returns whether `command` passes in `clone` under `environment`, writing
    the command and everything it prints to `out`.
    """
    print(f"$ {join(command)}", file=out, flush=True)
    return run(
        command,
        cwd    = clone,
        env    = environment,
        stderr = STDOUT,
        stdout = out
    ).returncode == 0


def main():
    """
    Prints the table, then tries and writes the lifts the flags ask for.
    """
    parser = ArgumentParser(prog="mise lift", description=__doc__.split("\n\n")[0])
    parser.add_argument("--hold", action="append", default=[], metavar="PIN")
    parser.add_argument("--try", action="store_true", dest="trial")
    parser.add_argument("--write", action="store_true")
    flags = parser.parse_args()

    lifts = table()
    print(rendered(lifts))
    if unknown := sorted(set(flags.hold) - {lift.name for lift in lifts}):
        parser.error(f"--hold names pins the table omits: {', '.join(unknown)}")

    holds = set(flags.hold)
    if flags.trial and any(lift.name not in holds for lift in lifts):
        holds = tried(holds, lifts)
    if flags.write and (moving := [lift for lift in lifts if lift.name not in holds]):
        write(moving)


def matched(path: str, pattern: str) -> str:
    """
    Returns the first capture of `pattern` in the file at `path`.
    """
    if found := search(pattern, read(path)):
        return found[1]
    raise SystemExit(f"no match for {pattern!r} in {path}")


def nested(keys: tuple[str, ...], mapping: dict) -> object:
    """
    Returns the value `keys` reach in `mapping`, or `None` where one of them
    is missing.
    """
    return reduce(
        lambda node, key: node.get(key) if isinstance(node, dict) else None,
        keys,
        mapping
    )


def ordered(version: str) -> Version:
    """
    Returns a key ordering `version` among the releases of its package,
    reading anything that is no PEP 440 version, such as an untagged commit,
    as `0`.
    """
    try:
        return Version(version)
    except InvalidVersion:
        return Version("0")


def output(*command: str, cwd: str | Path | None = None) -> str:
    """
    Returns everything `command` prints, stderr included, stopping the task
    with that output where the command exits nonzero.
    """
    ran = run([*command], cwd=cwd, stderr=STDOUT, stdout=PIPE, text=True)
    if ran.returncode:
        raise SystemExit(f"{join(command)} exited {ran.returncode}:\n{ran.stdout}")
    return ran.stdout


def packument(name: str) -> dict:
    """
    Returns the npm registry's document for the package `name`.
    """
    return fetched(f"https://registry.npmjs.org/{quote(name, safe='@')}")


def published_commit(crate: str, version: str) -> str:
    """
    Returns the commit the crate `version` was published from, which cargo
    records in `.cargo_vcs_info.json` inside the archive it uploads.
    """
    archive = f"https://static.crates.io/crates/{crate}/{crate}-{version}.crate"
    with open_tar(fileobj=BytesIO(downloaded(archive))) as tar:
        recorded = tar.extractfile(f"{crate}-{version}/.cargo_vcs_info.json")
        return from_json(recorded.read())["git"]["sha1"]


def read(path: str) -> str:
    """
    Returns the text of the file at `path`.
    """
    return Path(path).read_text(encoding="utf-8")


def toml(path: str) -> dict:
    """
    Returns the parsed TOML mapping at `path`.
    """
    return from_toml(read(path))


def release(repo: str) -> Release:
    """
    Returns the latest GitHub release of `repo`, its version being the tag
    the release names.
    """
    latest = from_json(output("gh", "api", f"repos/{repo}/releases/latest"))
    return Release(latest["tag_name"], stamp(latest["published_at"]))


def agreed(name: str, pins: list[Pin]) -> str:
    """
    Returns the version every one of `pins` is written at, stopping the task
    where two homes of the `name` row disagree.
    """
    if len(written := {pin.written for pin in pins}) != 1:
        homes = sorted(
            f"{pin.home.path if pin.home else 'a lockfile'} at {pin.written}"
            for pin in pins
        )
        raise SystemExit(f"the homes of {name} disagree: {', '.join(homes)}")
    return written.pop()


def cargo_pins() -> Iterator[Pin]:
    """
    Yields the pin of every crate a Cargo manifest of the workspace
    names with a version, the `ruff_*` crates under one row, beside
    the `rust-version` the workspace declares and the ruff release
    `[package.metadata.ruff]` names.
    """
    members = toml("Cargo.toml")["workspace"]["members"]
    for path in ["Cargo.toml", *(f"{member}/Cargo.toml" for member in members)]:
        manifest = toml(path)
        for keys in DEPENDENCIES:
            for crate, spec in (nested(keys, manifest) or {}).items():
                if version := spec.get("version") if isinstance(spec, dict) else spec:
                    yield Pin(
                        home    = Home(path, keys=(*keys, crate)),
                        name    = RUFF if crate.startswith(f"{RUFF}_") else crate,
                        package = crate,
                        source  = "cargo",
                        written = version.lstrip("=^~ ")
                    )
        for keys, name in [
            (("package", "metadata", RUFF, "release"), RUFF),
            (("package", "rust-version"), "rust"),
            (("workspace", "package", "rust-version"), "rust")
        ]:
            if isinstance(version := nested(keys, manifest), str):
                yield Pin(Home(path, keys=keys), name, name, "", version)


def mise_pins() -> list[Pin]:
    """
    Returns the pin of every tool the mise config declares, named without
    its backend and owner.
    """
    return [
        Pin(
            home    = Home(CONFIG, keys=("tools", tool)),
            name    = bare(tool),
            package = tool,
            source  = "mise",
            written = spec["version"] if isinstance(spec, dict) else spec
        )
        for tool, spec in toml(CONFIG)["tools"].items()
    ]


def pins() -> list[Pin]:
    """
    Returns every pin the checkout writes, read from the mise config, the
    Cargo manifests, the docs site's manifest, the Python project and each
    task script's PEP 723 block, the workflows and composite actions, and
    the README's Rust badge.
    """
    badge = ".github/README.md"
    return [
        *mise_pins(),
        *cargo_pins(),
        *site_pins(),
        *python_pins(),
        *workflow_pins(),
        Pin(
            home    = Home(badge, parts=2, pattern=r"(?m)^.*\brust\.svg\b.*$"),
            name    = "rust",
            package = "rust",
            source  = "",
            written = matched(badge, r"rust\.svg.*?!\[(\d+\.\d+)\+\]")
        )
    ]


def rendered(lifts: list[Lift]) -> str:
    """
    Returns the Markdown table listing `lifts`, each column padded to one
    width, or a sentence saying no pin trails its newest release.
    """
    if not lifts:
        return "Every pin stands at its newest release."

    header = [
        "**Pin**", "**Current**", "**Newest**",
        "**Major**", "**Days Old**", "**Alerts Cleared**"
    ]
    rows   = [header, *(lift.cells() for lift in lifts)]
    widths = [max(map(len, column)) for column in zip(*rows)]
    lines  = [
        "| " + " | ".join(cell.ljust(width) for cell, width in zip(row, widths)) + " |"
        for row in rows
    ]
    lines.insert(1, "|" + "|".join("-" * (width + 2) for width in widths) + "|")
    return "\n".join(lines)


def python_pins() -> Iterator[Pin]:
    """
    Yields the pin of every requirement the Python project names, an
    exact one written in `crate/pyproject.toml` and any other held by
    `crate/uv.lock` alone, beside every exact requirement a task script's
    PEP 723 block names.
    """
    path    = "crate/pyproject.toml"
    project = toml(path)
    locked  = {
        canonicalize_name(package["name"]): package["version"]
        for package in toml("crate/uv.lock")["package"]
        if "version" in package
    }
    for requirement in project["build-system"].get("requires", []):
        if pin := Pin.from_requirement(path, requirement, "pypi"):
            yield pin
    for requirement in [
        *project["project"].get("dependencies", []),
        *(
            entry for group in project.get("dependency-groups", {}).values()
            for entry in group if isinstance(entry, str)
        )
    ]:
        if not (pin := Pin.from_requirement(path, requirement, "uv")):
            name = canonicalize_name(Requirement(requirement).name)
            pin  = Pin(None, name, name, "uv", locked[name])
        yield pin

    scripts = output("git", "grep", "-l", "-e", "^# /// script$", "--", ".mise").split()
    for script in scripts:
        block    = search(SCRIPT_BLOCK, read(script))
        metadata = from_toml(sub(r"(?m)^# ?", "", block["content"]))
        for requirement in metadata.get("dependencies", []):
            if pin := Pin.from_requirement(script, requirement, "uv"):
                yield pin


def stamp(text: str | None) -> datetime | None:
    """
    Returns the time an ISO 8601 `text` names, read as UTC where it names no
    zone, or `None` where there is no text.
    """
    if not text:
        return None
    parsed = datetime.fromisoformat(text)
    return parsed if parsed.tzinfo else parsed.replace(tzinfo=UTC)


def table() -> list[Lift]:
    """
    Returns one row per name whose pins trail their newest release, sorted
    by name, each carrying the open Dependabot alerts its lift clears.
    """
    found   = pins()
    reports = Reports.read(sorted({pin.script for pin in found} - {None}))
    grouped = defaultdict(list)
    for pin in found:
        grouped[pin.name].append(pin)

    rows = (Lift.of(name, each, reports) for name, each in sorted(grouped.items()))
    if not (lifts := [*filter(None, rows)]):
        return []
    open_alerts = alerts()
    return [lift.clearing(open_alerts) for lift in lifts]


def tagged(repo: str) -> dict[str, str]:
    """
    Maps every tag of the GitHub `repo` to the commit it names, reading an
    annotated tag as the commit beneath it. `ls-remote` lists that commit
    under the tag's name followed by `^{}`, which sorts after the tag
    itself.
    """
    listed = output("git", "ls-remote", "--tags", f"https://github.com/{repo}")
    refs   = sorted(
        (ref.removeprefix("refs/tags/"), sha)
        for sha, ref in (line.split("\t") for line in listed.splitlines())
    )
    return {tag.removesuffix("^{}"): sha for tag, sha in refs}


def site_pins() -> list[Pin]:
    """
    Returns the pin of every package the docs site's manifest declares,
    of each `overrides` entry, which no report covers where it names no
    declared package, and of each `patchedDependencies` key.
    """
    path     = "site/package.json"
    manifest = from_json(read(path))
    declared = manifest.get("dependencies", {}) | manifest.get("devDependencies", {})
    entries  = [
        *((name, version, "bun") for name, version in declared.items()),
        *(
            (name, version, "bun" if name in declared else "npm")
            for name, version in manifest.get("overrides", {}).items()
        )
    ]
    return [
        *(
            Pin(
                Home(path, pattern=rf'"{escape(name)}"\s*:\s*"[^"]*"'),
                name,
                name,
                source,
                version
            )
            for name, version, source in entries
        ),
        *(
            Pin(Home(path, pattern=rf'"{escape(key)}"'), name, name, "bun", version)
            for key in manifest.get("patchedDependencies", {})
            for name, _, version in [key.rpartition("@")]
        )
    ]


def workflow_pins() -> Iterator[Pin]:
    """
    Yields the pin of every third-party action a workflow or composite
    action uses by commit, named for its repository, beside each input in
    `INPUTS` that names a tool's version.
    """
    for path in sorted(map(str, Path(".github").rglob("*.y*ml"))):
        text = read(path)
        for match in finditer(USES, text):
            repo = match["repo"]
            home = Home(path, pattern=rf"{escape(repo)}(?:/[\w./-]*)?@[0-9a-f]{{40}}")
            yield Pin(home, repo, repo, "action", match["written"])
        for key, (name, source) in INPUTS.items():
            for match in finditer(rf'\b{key}\s*:\s*"?v?(\d[^"\s]*)"?', text):
                home = Home(path, pattern=rf"\b{key}\s*:\s*\S+")
                yield Pin(home, name, name, source, match[1])


def tried(holds: set[str], lifts: list[Lift]) -> set[str]:
    """
    Returns the holds of the first attempt whose `mise ci` passes, trying
    every lift but `holds` first and then holding back each major lift in
    turn, and stops the task where no attempt passes.
    """
    logs    = LOGS / datetime.now(UTC).strftime("%Y%m%dT%H%M%SZ")
    retries = [lift.name for lift in lifts if lift.major and lift.name not in holds]
    logs.mkdir(parents=True)
    for held in [None, *retries]:
        attempted = holds | {held} - {None}
        named     = f"holding back {held}" if held else "with every lift"
        log       = logs / (sub(r"[^\w.-]+", "-", named) + ".log")
        passed    = attempt(attempted, log)
        print(f"{'passed' if passed else 'failed'} {named}, logged at {log}")
        if passed:
            return attempted
    raise SystemExit("no attempt passed, so nothing is written")


def write(lifts: list[Lift]):
    """
    Writes each of `lifts` into every home of its pins, locks and installs
    the tools a lifted mise pin names, moves each lifted package in its
    manager's lockfile under the written toolset, and runs `mise relock`.
    """
    edits = defaultdict(set)
    for home, new, old in (edit for lift in lifts for edit in lift.edits):
        edits[home.path].add((home, new, old))

    for path, changes in sorted(edits.items()):
        text = read(path)
        for home, new, old in changes:
            text = home.rewrite(new, old, text)
        Path(path).write_text(text, encoding="utf-8")

    moved  = [pin for lift in lifts for pin in lift.pins]
    crates = {f"{pin.package}@{pin.written}" for pin in moved if pin.source == "cargo"}
    locked = {
        canonicalize_name(pin.package) for pin in moved
        if pin.source == "uv" and not pin.script
    }
    if any(pin.source == "mise" for pin in moved):
        output("mise", "lock")
        output("mise", "install")
    if crates:
        output("mise", "x", "--", "cargo", "update", *flagged("--package", crates))
    if locked:
        packages = flagged("--upgrade-package", locked)
        output("mise", "x", "--", "uv", "lock", "--project", "crate", *packages)
    print(output("mise", "run", "--force", "lock:sync"), end="")


if __name__ == "__main__":

    main()
