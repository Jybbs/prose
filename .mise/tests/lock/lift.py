"""
Pins the pins `lock:lift` reads from each home, the reports and registries
it reads their newest releases from, the releases mise's cutoff holds back,
the rows and the tables it prints, the holds it accepts, the attempts its
trial runs, and what its write changes.
"""

from collections.abc   import Callable
from datetime          import UTC, datetime, timedelta
from gzip              import compress
from io                import BytesIO, StringIO
from json              import dumps
from pathlib           import Path
from pytest            import CaptureFixture, MonkeyPatch, fixture, mark, param, raises
from pytest_subprocess import FakeProcess
from subprocess        import run
from tarfile           import TarInfo, open as tar_open
from types             import ModuleType

CRATES   = "https://crates.io/api/v1/crates/toml"
NEW      = "b" * 40
OLD      = "a" * 40
PYPI     = "https://pypi.org/pypi/toml"
SCRIPTS  = ["git", "grep", "-l", "-e", "^# /// script$", "--", ".mise"]
SYNC     = ["mise", "run", "--force", "lock:sync"]
UNTAGGED = "c" * 40
TREE     = {
    ".github/README.md": (
        "[![Rust](rust.svg)![1.98+](https://img.shields.io/badge/1.98+-8a80cb)]"
        "(https://www.rust-lang.org/)\n"
        "[![Python](python.svg)![3.10+](https://img.shields.io/badge/3.10+-8a80cb)]\n"
    ),

    ".github/actions/wheel/action.yml":
        'with:\n  maturin-version : v1.15.0\n  wranglerVersion : "4.137.0"\n',

    ".github/workflows/ci.yml": (
        f"steps:\n  - uses : &restore actions/cache/restore@{OLD}\n"
        f"  - uses : actions/cache/save@{OLD}\n  - uses : $/.github/actions/wheel\n"
    ),

    ".mise/config.toml": (
        'min_version = "2026.9.18"\n\n'
        '[env]\n_.python.venv = { path = "crate/.venv" }\nPROBE = "on"\n\n[tools]\n'
        '"aqua:mitsuhiko/insta" = { version = "1.48.0", os = ["macos"] }\n'
        'maturin                = "1.15.0"\n'
        'rust                   = { version = "1.98.1", profile = "minimal" }\n'
    ),

    ".mise/tasks/repo/audit.py": (
        "#!/usr/bin/env -S uv run --script\n# /// script\n"
        '# requires-python = ">=3.11"\n'
        '# dependencies = ["PyYAML==6.0.3", "jinja2>=3"]\n'
        "# ///\n"
    ),

    "Cargo.toml": (
        '[workspace]\nmembers = ["crate"]\n\n[workspace.dependencies]\n'
        'ruff_python_ast = "=0.0.5"\nruff_text_size  = "=0.0.5"\n'
        'serde           = { version = "1.0.229", features = ["derive"] }\n\n'
        '[workspace.package]\nrust-version = "1.98.1"\n\n'
        "[profile.dev.package]\ninsta = { opt-level = 2 }\n"
    ),

    "crate/Cargo.toml": (
        '[package]\nname                   = "prose"\nrust-version.workspace = true\n\n'
        '[package.metadata.ruff]\nrelease = "0.15.22"\n\n'
        "[dependencies]\nserde        = { workspace = true }\n"
        'wasm-bindgen = "=0.2.128"\n\n'
        '[dev-dependencies]\ninsta = { version = "1.48.0", features = ["glob"] }\n'
    ),

    "crate/pyproject.toml": (
        '[build-system]\nrequires = ["maturin==1.15.0", "setuptools>=61"]\n\n'
        '[project]\nname = "prose-formatter"\ndependencies = ["tomli==2.2.1"]\n\n'
        '[dependency-groups]\ntest = ["pytest", { include-group = "other" }]\n'
    ),

    "crate/uv.lock": (
        '[[package]]\nname   = "prose-formatter"\nsource = { editable = "." }\n\n'
        '[[package]]\nname    = "pytest"\nversion = "9.1.1"\n'
    ),

    "site/package.json": (
        '{\n  "overrides": {\n    "shiki" : "4.4.3",\n    "vite"  : "8.3.0"\n  },\n'
        '  "devDependencies": {\n    "vite" : "8.3.0"\n  },\n'
        '  "patchedDependencies": {\n    "vite@8.3.0": "patches/vite.patch"\n  }\n}\n'
    )
}
WHEN = datetime(2026, 10, 1, tzinfo=UTC)


@fixture
def lift(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Returns the loaded `lock:lift` script.
    """
    return task("lock/lift")


@fixture
def released(lift: ModuleType, monkeypatch: MonkeyPatch) -> list[tuple]:
    """
    Returns the list each registry lookup of a release is recorded in, every
    lookup answering the version it was asked for, published at `WHEN`.
    """
    asked = []

    def lookup(pin: object, version: str | None = None) -> object:
        """
        Records the package and version a release is looked up for and
        returns it, published at `WHEN`.
        """
        asked.append((pin.package, version))
        return lift.Release(version, WHEN)

    monkeypatch.setattr(lift.Pin, "released", lookup)
    return asked


@fixture
def tree(workdir: Callable[[dict[str, str]], Path]) -> Path:
    """
    Returns a working tree holding every kind of home `lock:lift` reads, and
    makes it the current directory.
    """
    return workdir(TREE)


def pin(
    lift    : ModuleType,
    name    : str,
    source  : str,
    written : str,
    **home
) -> object:
    """
    Returns a pin of the package `name` at `written`, whose home is the file
    and keys or pattern `home` names, or none where `home` is empty.
    """
    return lift.Pin(lift.Home(**home) if home else None, name, name, source, written)


def row(
    lift    : ModuleType,
    name    : str,
    *,
    current : str,
    newest  : str,
    **fields
) -> object:
    """
    Returns a row lifting the package `name` from `current` to `newest`,
    published at `WHEN`.
    """
    return lift.Lift(
        **{
            "current"   : current,
            "name"      : name,
            "newest"    : newest,
            "published" : WHEN,
            "pins"      : (),
            "targets"   : {current: newest},
            **fields
        }
    )


@mark.parametrize(
    ("commit", "latest", "lifted"),
    [
        param(OLD, "v5.0.1", None, id = "current"),
        param(OLD, "v5.1.2", ("v5.0.1", "v5.1.2", {OLD: NEW}, False), id = "behind"),
        param(
            UNTAGGED,
            "v5.1.2",
            (UNTAGGED[:12], "v5.1.2", {UNTAGGED: NEW}, True),
            id = "untagged"
        )
    ]
)
def test_action_moves_the_pinned_commit_to_the_latest_release(
    commit      : str,
    latest      : str,
    lift        : ModuleType,
    lifted      : tuple | None,
    monkeypatch : MonkeyPatch
):
    """
    Pins that an action's row names the most specific tag on the commit it
    pins, or the commit's first twelve digits where no tag names it, and the
    latest release's tag, moving the commit to the one that release tags,
    and that an action pinned at its latest release takes no row.
    """
    pinned = [pin(
        lift    = lift,
        name    = "jdx/mise-action",
        source  = "action",
        written = commit
    )]
    monkeypatch.setattr(lift, "release", lambda repo: lift.Release(latest, WHEN))
    monkeypatch.setattr(
        lift,
        "tagged",
        lambda repo: {"v5": NEW, "v5.0.1": OLD, "v5.1.2": NEW}
    )

    found = lift.Lift.of("jdx/mise-action", pinned, lift.Reports({}))

    assert (found and (
        found.current, found.newest, found.targets, found.major
    )) == lifted


@mark.parametrize(
    ("wrote", "checked", "passed", "logged"),
    [
        param(0, 0, True, ["wrote", "$ mise run ci", "checked"], id = "pass"),
        param(0, 1, False, ["wrote", "$ mise run ci", "checked"], id = "ci fails"),
        param(1, 0, False, ["wrote"], id = "write fails")
    ]
)
def test_attempt_runs_ci_in_a_clone_carrying_the_checkout(
    checked     : int,
    fp          : FakeProcess,
    lift        : ModuleType,
    logged      : list[str],
    monkeypatch : MonkeyPatch,
    passed      : bool,
    tmp_path    : Path,
    tree        : Path,
    wrote       : int
):
    """
    Pins that an attempt clones the checkout at its `HEAD`, carries its
    changes into the clone, writes there every lift but its holds, and runs
    `mise ci`, logging each command and its output, that the attempt passes
    exactly where `mise ci` does, and that a write that fails ends the
    attempt before `mise ci` runs.
    """
    monkeypatch.setattr(lift, "isolated", lambda clone, root: {"PATH": "/bin"})
    fp.register(["git", "clone", "--quiet", "--shared", str(tree), fp.any()])
    fp.register(["git", "diff", "--name-only", "--no-renames", "-z", "HEAD"])
    fp.register(["git", "ls-files", "--exclude-standard", "--others", "-z"])
    fp.register(
        ["mise", "run", "lock:lift", "--write", "--hold", "ruff"],
        returncode = wrote,
        stdout     = "wrote\n"
    )
    fp.register(["mise", "run", "ci"], returncode=checked, stdout="checked\n")
    log = tmp_path / "attempt.log"

    assert lift.attempt({"ruff"}, log) is passed
    assert log.read_text().splitlines() == [
        "$ mise run lock:lift --write --hold ruff",
        *logged
    ]


def test_carry_copies_every_change_and_untracked_file(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    tmp_path    : Path
):
    """
    Pins that the clone receives each file the checkout changes from `HEAD`
    and each untracked file git does not ignore, and loses each tracked file
    the checkout deleted, while an ignored file stays behind.
    """
    root, clone = tmp_path / "root", tmp_path / "clone"
    root.mkdir()
    for name, text in {
        ".gitignore" : "ignored\n",
        "changed"    : "old\n",
        "gone"       : "x\n"
    }.items():
        (root / name).write_text(text)
    for variable in ("GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"):
        monkeypatch.delenv(variable, raising=False)
    monkeypatch.chdir(root)
    identity = ["-c", "user.email=a@b", "-c", "user.name=a"]
    for command in [["init", "-q"], ["add", "."], [*identity, "commit", "-qm", "base"]]:
        run(["git", *command], check=True)
    run(["git", "clone", "-q", str(root), str(clone)], check=True)
    (root / "changed").write_text("new\n")
    (root / "gone").unlink()
    (root / "ignored").write_text("i\n")
    (root / "fresh").mkdir()
    (root / "fresh/new.txt").write_text("n\n")

    lift.carry(clone, root)

    assert (clone / "changed").read_text() == "new\n"
    assert (clone / "fresh/new.txt").read_text() == "n\n"
    assert not (clone / "gone").exists()
    assert not (clone / "ignored").exists()


@mark.parametrize(
    ("age", "published", "cleared"),
    [
        param("24h", WHEN, WHEN + timedelta(hours=24), id = "hours"),
        param("2w", WHEN, WHEN + timedelta(weeks=2), id = "weeks"),
        param("90s", WHEN, WHEN + timedelta(seconds=90), id = "seconds"),
        param("30m", WHEN, WHEN + timedelta(minutes=30), id = "minutes"),
        param("6mo", WHEN, None, id = "months"),
        param("1y", WHEN, None, id = "year"),
        param("2024-06-01", WHEN, None, id = "date"),
        param("24h", None, None, id = "unreported")
    ]
)
def test_cleared_counts_a_cutoff_of_seconds_through_weeks_from_publication(
    age       : str,
    cleared   : datetime | None,
    lift      : ModuleType,
    published : datetime | None
):
    """
    Pins that a cutoff counted in seconds, minutes, hours, days, or weeks
    clears a release that long after it was published, and that a cutoff
    given as a date or a longer span, or a release reporting no time, has no
    clearing time.
    """
    assert lift.Release("1.99.1", published).cleared(age) == cleared


def test_clearing_matches_a_python_package_across_its_spellings(lift: ModuleType):
    """
    Pins that a pip alert on `PyYAML` clears for a pin written `pyyaml`,
    since PyPI reads both as one name.
    """
    alert = {
        "number"                 : 3,
        "dependency"             : {"package": {"ecosystem": "pip", "name": "PyYAML"}},
        "security_vulnerability" : {"first_patched_version": {"identifier": "6.0.2"}}
    }
    lifted = row(
        current = "6.0.1",
        lift    = lift,
        name    = "pyyaml",
        newest  = "6.0.3",
        pins    = (pin(
            lift    = lift,
            name    = "pyyaml",
            source  = "uv",
            written = "6.0.1"
        ),)
    )

    assert lifted.clearing([alert]).alerts == (3,)


def test_clearing_names_each_alert_the_newest_release_patches(lift: ModuleType):
    """
    Pins that a row carries an open alert on one of its packages whose first
    patched version is the newest release or one before it, and none on
    another package, on a patch the release falls short of, or with no patch
    at all.
    """
    def alert(name: str, number: int, patched: str | None) -> dict:
        """
        Returns the fields of an npm alert `lock:lift` reads.
        """
        return {
            "number"                 : number,
            "dependency"             : {"package": {"ecosystem": "npm", "name": name}},
            "security_vulnerability" : {
                "first_patched_version": patched and {"identifier": patched}
            }
        }

    pins = (pin(
        lift    = lift,
        name    = "vite",
        source  = "bun",
        written = "8.3.0"
    ),)
    lifted = row(
        current = "8.3.0",
        lift    = lift,
        name    = "vite",
        newest  = "8.3.4",
        pins    = pins
    )
    alerts = [
        alert("vite", 1, "8.3.2"),
        alert("vite", 2, "9.0.0"),
        alert("vue", 3, "1.0.0"),
        alert("vite", 4, None),
        alert("vite", 5, "8.3.4")
    ]

    assert lifted.clearing(alerts).alerts == (1, 5)


@mark.parametrize(
    ("result", "age"),
    [
        param({"stdout": "7d\n"}, "7d", id = "set"),
        param({"returncode": 1}, "24h", id = "unset")
    ]
)
def test_cutoff_reads_the_setting_or_takes_the_default(
    age    : str,
    fp     : FakeProcess,
    lift   : ModuleType,
    result : dict
):
    """
    Pins that the cutoff is the `minimum_release_age` mise's settings name,
    and mise's `24h` default where `mise settings get` finds none set, read
    once however many tools ask for it.
    """
    fp.register(["mise", "settings", "get", "minimum_release_age"], **result)

    assert (lift.cutoff(), lift.cutoff()) == (age, age)


@mark.parametrize(
    ("spec", "age", "asked"),
    [
        param('rust = "1.99.0"\n', timedelta(hours=24), 1, id = "setting"),
        param(
            'rust = { version = "1.99.0", minimum_release_age = "7d" }\n',
            timedelta(days=7),
            0,
            id = "tool option"
        )
    ]
)
def test_held_lists_each_stable_release_newer_than_the_report(
    age     : timedelta,
    asked   : int,
    fp      : FakeProcess,
    lift    : ModuleType,
    spec    : str,
    workdir : Callable[[dict[str, str]], Path]
):
    """
    Pins that a tool holds back each stable release newer than the one its
    report gives, read with the cutoff off and clearing at its publication
    plus the tool's own cutoff where its spec sets one and the setting
    otherwise, and that an older release, a prerelease, and a nightly stay
    out.
    """
    workdir({".mise/config.toml": f"[tools]\n{spec}"})
    listed = [
        {"created_at": "2026-09-01T00:00:00Z", "version": "1.99.0"},
        {"created_at": "2026-10-01T00:00:00Z", "version": "1.99.1"},
        {"created_at": "2026-10-01T00:00:00Z", "version": "2.0.0-beta.1"},
        {"created_at": "2026-10-01T00:00:00Z", "version": "nightly-2026-10-01"},
        {"version": "1.99.2"}
    ]
    settings = ["mise", "settings", "get", "minimum_release_age"]
    fp.register(
        ["mise", "ls-remote", "--json", "--minimum-release-age", "0s", "rust"],
        stdout = dumps(listed)
    )
    fp.register(settings, occurrences=asked, returncode=1)

    tool = pin(
        lift    = lift,
        name    = "rust",
        source  = "mise",
        written = "1.98.1"
    )

    assert tool.held("1.99.0") == [
        lift.Held(WHEN + age, "rust", "1.99.1"),
        lift.Held(None, "rust", "1.99.2")
    ]
    assert fp.call_count(settings) == asked


def test_held_lists_nothing_at_the_newest_stable_release(
    fp   : FakeProcess,
    lift : ModuleType
):
    """
    Pins that a tool whose report gives its newest stable release holds
    nothing back, newer prereleases included, and never asks mise for the
    cutoff.
    """
    listed = [
        {"created_at": "2026-10-01T00:00:00Z", "version": "1.99.1"},
        {"created_at": "2026-10-01T00:00:00Z", "version": "2.0.0-beta.1"}
    ]
    fp.register(
        ["mise", "ls-remote", "--json", "--minimum-release-age", "0s", "rust"],
        stdout = dumps(listed)
    )

    tool = pin(
        lift    = lift,
        name    = "rust",
        source  = "mise",
        written = "1.99.1"
    )

    assert tool.held("1.99.1") == []


def test_isolated_drops_what_the_checkout_set(
    fp          : FakeProcess,
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    tree        : Path
):
    """
    Pins that a clone's commands run without the variables the checkout's
    mise config and task runner set and without each `PATH` entry inside
    the checkout, trusting the clone's config and naming the repository
    `gh` reads.
    """
    given = {
        "HOME"              : "/home",
        "MISE_JOBS"         : "4",
        "MISE_PROJECT_ROOT" : str(tree),
        "MISE_TASK_NAME"    : "lock:lift",
        "PATH"              : f"{tree}/.mise/bin:/usr/bin",
        "PROBE"             : "on",
        "VIRTUAL_ENV"       : f"{tree}/crate/.venv"
    }
    for name, value in given.items():
        monkeypatch.setenv(name, value)
    fp.register(
        ["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"],
        stdout = "Jybbs/prose\n"
    )

    environment = lift.isolated(Path("/clone"), tree)

    assert {name: environment.get(name) for name in [*given, "GH_REPO"]} == {
        "GH_REPO"           : "Jybbs/prose",
        "HOME"              : "/home",
        "MISE_JOBS"         : "4",
        "MISE_PROJECT_ROOT" : None,
        "MISE_TASK_NAME"    : None,
        "PATH"              : "/usr/bin",
        "PROBE"             : None,
        "VIRTUAL_ENV"       : None
    }
    assert environment["MISE_TRUSTED_CONFIG_PATHS"] == "/clone"


def test_isolated_names_no_repository_gh_cannot_resolve(
    fp          : FakeProcess,
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    tree        : Path
):
    """
    Pins that a checkout whose remote `gh` cannot read passes no `GH_REPO`
    on to the clone, leaving its own unset.
    """
    monkeypatch.delenv("GH_REPO", raising=False)
    fp.register(
        ["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"],
        returncode = 1
    )

    assert "GH_REPO" not in lift.isolated(Path("/clone"), tree)


@mark.parametrize(
    ("current", "newest", "major"),
    [
        param("0.33.1", "0.34.0", True, id = "zero major"),
        param("2026.7.0", "2026.9.0", False, id = "calendar"),
        param("0.0.5", "0.0.17", True, id = "patch leads"),
        param("1.20.0", "2.0.0", True, id = "major"),
        param("v5.0.1", "v5.1.2", False, id = "tag"),
        param("3.14.6", "3.15.0", False, id = "minor")
    ]
)
def test_major_reads_the_left_most_non_zero_part(
    current : str,
    lift    : ModuleType,
    major   : bool,
    newest  : str
):
    """
    Asserts that a lift is major exactly where it changes the left-most
    non-zero part of the version.
    """
    assert row(
        current = current,
        lift    = lift,
        name    = "tool",
        newest  = newest
    ).major is major


def test_main_exits_2_on_a_hold_the_table_omits(
    capsys      : CaptureFixture[str],
    lift        : ModuleType,
    monkeypatch : MonkeyPatch
):
    """
    Pins that a `--hold` naming no row of the table exits 2 once the table
    prints, before any trial or write.
    """
    argv = ["lift", "--try", "--write", "--hold", "vite", "--hold", "nope"]
    ran  = []
    monkeypatch.setattr(
        lift,
        "table",
        lambda: (
            [row(
                current = "1",
                lift    = lift,
                name    = "vite",
                newest  = "2"
            )],
            []
        )
    )
    monkeypatch.setattr(lift, "tried", lambda *args: ran.append("tried"))
    monkeypatch.setattr(lift, "write", lambda *args: ran.append("write"))
    monkeypatch.setattr("sys.argv", argv)

    with raises(SystemExit) as stopped:
        lift.main()

    printed = capsys.readouterr()
    assert (stopped.value.code, ran) == (2, [])
    assert "`vite`" in printed.out
    assert "--hold names pins the table omits: nope" in printed.err


@mark.parametrize(
    "held",
    [
        param([], id = "none held"),
        param([("rust", "1.99.1")], id = "held")
    ]
)
def test_main_lists_the_held_releases_apart_from_the_lifts(
    capsys      : CaptureFixture[str],
    held        : list[tuple[str, str]],
    lift        : ModuleType,
    monkeypatch : MonkeyPatch
):
    """
    Pins that the releases the cutoff holds back print in a table of their
    own below the lifts, and that nothing follows the lifts where none is
    held.
    """
    waiting = [lift.Held(WHEN, name, version) for name, version in held]
    monkeypatch.setattr(lift, "table", lambda: ([], waiting))
    monkeypatch.setattr("sys.argv", ["lift"])

    lift.main()

    tail = f"\n{lift.waiting(waiting)}\n" if waiting else ""
    assert capsys.readouterr().out == f"{lift.rendered([])}\n{tail}"


def test_main_writes_the_lifts_the_passing_attempt_kept(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch
):
    """
    Pins that `--try --write` writes every lift but the holds of the attempt
    that passed, which add a held-back major lift to the given ones.
    """
    lifted = [row(
        current = "1.0.0",
        lift    = lift,
        name    = name,
        newest  = "2.0.0"
    ) for name in ("ruff", "serde", "vite")]
    written = []
    monkeypatch.setattr(lift, "table", lambda: (lifted, []))
    monkeypatch.setattr(lift, "tried", lambda holds, lifts: holds | {"ruff"})
    monkeypatch.setattr(
        lift,
        "write",
        lambda lifts: written.extend(each.name for each in lifts)
    )
    monkeypatch.setattr("sys.argv", ["lift", "--try", "--write", "--hold", "vite"])

    lift.main()

    assert written == ["serde"]


def test_of_lifts_every_home_of_a_row_to_its_newest_release(
    lift     : ModuleType,
    released : list[tuple]
):
    """
    Pins that a row's newest release comes from the report covering its
    sourced pin, that its time is looked up for that pin, and that each home
    moves to the newest release as it writes it, the badge taking its first
    two components.
    """
    pins = [
        pin(
            keys    = ("tools", "rust"),
            lift    = lift,
            name    = "rust",
            path    = "config",
            source  = "mise",
            written = "1.98.1"
        ),
        pin(
            keys    = ("rust-version",),
            lift    = lift,
            name    = "rust",
            path    = "Cargo.toml",
            source  = "",
            written = "1.98.1"
        ),
        pin(
            lift    = lift,
            name    = "rust",
            parts   = 2,
            path    = "README.md",
            pattern = "x",
            source  = "",
            written = "1.98"
        )
    ]

    lifted = lift.Lift.of(
        "rust",
        pins,
        lift.Reports({("mise", "rust", "1.98.1"): "1.99.0"})
    )

    assert (
        lifted.current, lifted.newest, lifted.published
    ) == ("1.98.1", "1.99.0", WHEN)
    assert lifted.targets == {"1.98": "1.99", "1.98.1": "1.99.0"}
    assert released == [("rust", "1.99.0")]


def test_of_looks_up_a_pin_no_report_covers(lift: ModuleType, monkeypatch: MonkeyPatch):
    """
    Pins that an `overrides` entry naming no declared package takes the npm
    registry's `latest` tag and its time.
    """
    document = {
        "dist-tags" : {"latest": "4.5.0"},
        "time"      : {"4.5.0": "2026-10-01T00:00:00.000Z"}
    }
    monkeypatch.setattr(lift, "packument", lambda name: document)

    lifted = lift.Lift.of(
        "shiki",
        [pin(
            lift    = lift,
            name    = "shiki",
            source  = "npm",
            written = "4.4.3"
        )],
        lift.Reports({})
    )

    assert (lifted.newest, lifted.published) == ("4.5.0", WHEN)


def test_of_returns_a_row_only_behind_the_newest_release(
    lift     : ModuleType,
    released : list[tuple]
):
    """
    Pins that a pin its report omits stands at its newest release and takes
    no row, whereas one its report names a newer release for takes one.
    """
    pinned = [pin(
        lift    = lift,
        name    = "serde",
        source  = "cargo",
        written = "1.0.229"
    )]
    behind = lift.Reports({("cargo", "serde", "1.0.229"): "1.0.230"})

    assert lift.Lift.of("serde", pinned, lift.Reports({})) is None
    assert lift.Lift.of("serde", pinned, behind).newest == "1.0.230"


def test_of_stops_where_the_homes_of_a_row_disagree(lift: ModuleType):
    """
    Pins that two sourced homes of one row written at different versions
    stop the task naming both.
    """
    pins = [
        pin(
            keys    = ("tools",),
            lift    = lift,
            name    = "insta",
            path    = "config",
            source  = "mise",
            written = "1.48.0"
        ),
        pin(
            lift    = lift,
            name    = "insta",
            source  = "cargo",
            written = "1.47.0"
        )
    ]

    with raises(SystemExit) as stopped:
        lift.Lift.of("insta", pins, lift.Reports({}))

    assert stopped.value.code == (
        "the homes of insta disagree: a lockfile at 1.47.0, config at 1.48.0"
    )


def test_output_stops_with_everything_a_failing_command_printed(
    fp   : FakeProcess,
    lift : ModuleType
):
    """
    Pins that a command exiting nonzero stops the task with the command, its
    exit status, and everything it printed.
    """
    fp.register(
        ["cargo", "update"],
        returncode = 101,
        stdout     = "error: no matching package\n"
    )

    with raises(SystemExit) as stopped:
        lift.output("cargo", "update")

    assert stopped.value.code == (
        "cargo update exited 101:\nerror: no matching package\n"
    )


@mark.parametrize(
    ("text", "home", "old", "new", "written"),
    [
        param(
            'ruff_python_ast = "=0.0.5"\nruff_text_size  = "=0.0.5"\n',
            {"keys": ("ruff_python_ast",)},
            "0.0.5",
            "0.0.17",
            'ruff_python_ast = "=0.0.17"\nruff_text_size  = "=0.0.5"\n',
            id = "operator"
        ),
        param(
            '[tools]\nrust = { version = "1.98.1", profile = "minimal" }\n',
            {"keys": ("tools", "rust")},
            "1.98.1",
            "1.99.0",
            '[tools]\nrust = { version = "1.99.0", profile = "minimal" }\n',
            id = "table"
        ),
        param(
            '  "vite"  : "8.3.0"\n  "vitest" : "8.3.0"\n',
            {"pattern": r'"vite"\s*:\s*"[^"]*"'},
            "8.3.0",
            "8.3.4",
            '  "vite"  : "8.3.4"\n  "vitest" : "8.3.0"\n',
            id = "pattern"
        ),
        param(
            "![1.98+](badge/1.98+) and 11.98\n",
            {"pattern": r"(?m)^.*$"},
            "1.98",
            "1.99",
            "![1.99+](badge/1.99+) and 11.98\n",
            id = "bounded"
        )
    ]
)
def test_rewrite_moves_the_pin_alone(
    home    : dict,
    lift    : ModuleType,
    new     : str,
    old     : str,
    text    : str,
    written : str
):
    """
    Pins that a home moves its own pin and keeps every other byte, an
    operator ahead of the version and a neighbor's aligned layout included,
    and that a pattern home moves only a version standing apart from other
    digits.
    """
    assert lift.Home("file", **home).rewrite(new, old, text) == written


def test_packument_escapes_a_scoped_package(lift: ModuleType, monkeypatch: MonkeyPatch):
    """
    Pins that a scoped npm package's `/` is escaped in the registry path
    while its `@` stays.
    """
    monkeypatch.setattr(lift, "fetched", lambda url: url)

    assert lift.packument(
        "@iconify-json/logos"
    ) == "https://registry.npmjs.org/@iconify-json%2Flogos"


def test_pins_reads_every_home(fp: FakeProcess, lift: ModuleType, tree: Path):
    """
    Pins every pin read from a tree holding one home of each kind, naming
    a mise tool without its backend and owner, a `ruff_*` crate under
    `ruff`, an override no package declares as the npm registry's, the
    build requirement as PyPI's, an unpinned group requirement at its locked
    version, and an exact PEP 723 requirement, while the `min_version`
    floor, a profile table, a local action, an inexact requirement, and the
    Python badge stay out.
    """
    fp.register(SCRIPTS, stdout=".mise/tasks/repo/audit.py\n")

    found = {
        (
            each.name, each.package, each.source, each.written,
            each.home and each.home.path
        )
        for each in lift.pins()
    }

    assert found == {
        ("actions/cache", "actions/cache", "action", OLD, ".github/workflows/ci.yml"),
        ("insta", "aqua:mitsuhiko/insta", "mise", "1.48.0", ".mise/config.toml"),
        ("insta", "insta", "cargo", "1.48.0", "crate/Cargo.toml"),
        ("maturin", "maturin", "", "1.15.0", ".github/actions/wheel/action.yml"),
        ("maturin", "maturin", "mise", "1.15.0", ".mise/config.toml"),
        ("maturin", "maturin", "pypi", "1.15.0", "crate/pyproject.toml"),
        ("pytest", "pytest", "uv", "9.1.1", None),
        ("pyyaml", "PyYAML", "uv", "6.0.3", ".mise/tasks/repo/audit.py"),
        ("ruff", "ruff", "", "0.15.22", "crate/Cargo.toml"),
        ("ruff", "ruff_python_ast", "cargo", "0.0.5", "Cargo.toml"),
        ("ruff", "ruff_text_size", "cargo", "0.0.5", "Cargo.toml"),
        ("rust", "rust", "", "1.98", ".github/README.md"),
        ("rust", "rust", "", "1.98.1", "Cargo.toml"),
        ("rust", "rust", "mise", "1.98.1", ".mise/config.toml"),
        ("serde", "serde", "cargo", "1.0.229", "Cargo.toml"),
        ("shiki", "shiki", "npm", "4.4.3", "site/package.json"),
        ("tomli", "tomli", "uv", "2.2.1", "crate/pyproject.toml"),
        ("vite", "vite", "bun", "8.3.0", "site/package.json"),
        ("wasm-bindgen", "wasm-bindgen", "cargo", "0.2.128", "crate/Cargo.toml"),
        ("wrangler", "wrangler", "npm", "4.137.0", ".github/actions/wheel/action.yml")
    }


@mark.parametrize(
    ("passing", "attempts", "holds"),
    [
        param({"every"}, ["with every lift"], set(), id = "whole"),
        param(
            {"typescript"},
            ["with every lift", "holding back ruff", "holding back typescript"],
            {"typescript"},
            id = "retry"
        )
    ]
)
def test_tried_retries_with_each_major_lift_held_back_in_turn(
    attempts    : list[str],
    holds       : set[str],
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    passing     : set[str],
    tmp_path    : Path
):
    """
    Pins that a trial whose whole lift fails retries with each major lift
    held back in turn, skipping a minor lift and a given hold, stops at the
    first attempt that passes, and returns its holds, logging each attempt
    under its own name.
    """
    logged = []

    def attempt(attempted: set[str], log: Path) -> bool:
        """
        Records the attempt's log and passes where it holds back a lift
        `passing` names, or every lift passes.
        """
        logged.append(log.stem.replace("-", " "))
        return bool(attempted - {"uv"} & passing) or "every" in passing

    monkeypatch.setattr(lift, "LOGS", tmp_path)
    monkeypatch.setattr(lift, "attempt", attempt)
    lifted = [
        row(
            current = "0.15.22",
            lift    = lift,
            name    = "ruff",
            newest  = "0.17.0"
        ),
        row(
            current = "1.0.229",
            lift    = lift,
            name    = "serde",
            newest  = "1.0.230"
        ),
        row(
            current = "6.0.3",
            lift    = lift,
            name    = "typescript",
            newest  = "7.0.2"
        ),
        row(
            current = "0.12.17",
            lift    = lift,
            name    = "uv",
            newest  = "0.13.0"
        )
    ]

    assert lift.tried({"uv"}, lifted) == {"uv"} | holds
    assert logged == attempts


def test_published_commit_reads_the_archive_cargo_uploaded(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch
):
    """
    Pins that a crate's commit is the `sha1` cargo records in
    `.cargo_vcs_info.json` inside the archive crates.io serves.
    """
    archive = BytesIO()
    with tar_open(fileobj=archive, mode="w:gz") as tar:
        recorded  = dumps({"git": {"sha1": NEW}}).encode()
        info      = TarInfo("ruff_text_size-0.0.17/.cargo_vcs_info.json")
        info.size = len(recorded)
        tar.addfile(info, BytesIO(recorded))
    monkeypatch.setattr(lift, "downloaded", lambda url: archive.getvalue())

    assert lift.published_commit("ruff_text_size", "0.0.17") == NEW


def test_release_reads_the_latest_tag_and_time(fp: FakeProcess, lift: ModuleType):
    """
    Pins that an action's latest release is the tag and the time `gh` reads
    off the repository's latest GitHub release.
    """
    fp.register(
        ["gh", "api", "repos/owner/action/releases/latest"],
        stdout = dumps({"published_at": "2026-10-01T00:00:00Z", "tag_name": "v2.0.0"})
    )

    assert lift.release("owner/action") == lift.Release("v2.0.0", WHEN)


def test_released_reads_the_time_mise_lists_for_a_tool(
    fp   : FakeProcess,
    lift : ModuleType
):
    """
    Pins that a tool's release time is the `created_at` mise lists beside
    that version with its `minimum_release_age` cutoff off, that a version
    listed without one reports no time, and that one listing serves every
    version read off the tool.
    """
    listed = [
        {"created_at": "2026-10-01T00:00:00.0Z", "version": "1.99.0"},
        {"version": "3.15.0"}
    ]
    remote = ["mise", "ls-remote", "--json", "--minimum-release-age", "0s", "rust"]
    fp.register(remote, stdout=dumps(listed))

    tool = pin(
        lift    = lift,
        name    = "rust",
        source  = "mise",
        written = "1.98.1"
    )

    assert tool.released("1.99.0") == lift.Release("1.99.0", WHEN)
    assert tool.released("3.15.0") == lift.Release("3.15.0")
    assert fp.call_count(remote) == 1


@mark.parametrize(
    ("source", "version", "asked"),
    [
        param(
            "cargo",
            "1.1.8+spec-1.1.0",
            [f"{CRATES}/1.1.8%2Bspec-1.1.0"],
            id = "crate"
        ),
        param("bun", "1.1.8+spec-1.1.0", [], id = "npm at a version"),
        param("npm", None, [], id = "npm newest"),
        param(
            "uv",
            "1.1.8+spec-1.1.0",
            [f"{PYPI}/1.1.8+spec-1.1.0/json"],
            id = "pypi at a version"
        ),
        param("pypi", None, [f"{PYPI}/json"], id = "pypi newest")
    ]
)
def test_released_reads_the_release_from_its_registry(
    asked       : list[str],
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    source      : str,
    version     : str | None
):
    """
    Pins that a release comes from crates.io for a crate, its version
    escaped in the path, from the npm registry's `time` map for an npm
    package, its `latest` tag standing in for no version, and from the
    earliest file PyPI lists for a Python package, PyPI's newest release
    standing in for no version.
    """
    newest = "1.1.8+spec-1.1.0"
    stamp  = "2026-10-01T00:00:00Z"
    files  = {
        "urls": [
            {"upload_time_iso_8601": "2026-10-02T00:00:00Z"},
            {"upload_time_iso_8601": stamp}
        ]
    }
    documents = {
        f"{CRATES}/1.1.8%2Bspec-1.1.0"  : {"version": {"created_at": stamp}},
        f"{PYPI}/1.1.8+spec-1.1.0/json" : {"info": {"version": newest}, **files},
        f"{PYPI}/json": {"info": {"version": newest}, **files}
    }
    fetched = []
    monkeypatch.setattr(lift, "sleep", lambda seconds: None)
    monkeypatch.setattr(
        lift,
        "fetched",
        lambda url: fetched.append(url) or documents[url]
    )
    monkeypatch.setattr(
        lift,
        "packument",
        lambda name: {"dist-tags": {"latest": newest}, "time": {newest: stamp}}
    )

    found = pin(
        lift    = lift,
        name    = "toml",
        source  = source,
        written = "1.1.6"
    ).released(version)

    assert found == lift.Release(newest, WHEN)
    assert fetched == asked


def test_rendered_pads_every_column_to_one_width(lift: ModuleType):
    """
    Pins the table's header, divider, and rows, each column padded to its
    widest cell, and the sentence printed where nothing trails its newest
    release.
    """
    lifted = [
        row(
            alerts    = (7, 9),
            current   = "0.12.17",
            lift      = lift,
            name      = "uv",
            newest    = "0.13.0",
            published = datetime.now(UTC) - timedelta(days=4)
        ),
        row(
            current   = "3.14.6",
            lift      = lift,
            name      = "python",
            newest    = "3.15.0",
            published = None
        )
    ]

    assert lift.rendered(lifted).splitlines() == [
        "| **Pin**  | **Current** | **Newest** | **Major** "
        "| **Days Old** | **Alerts Cleared** |",
        "|----------|-------------|------------|-----------"
        "|--------------|--------------------|",
        "| `uv`     | `0.12.17`   | `0.13.0`   | yes       "
        "| 4            | #7, #9             |",
        "| `python` | `3.14.6`    | `3.15.0`   | no        "
        "|              |                    |"
    ]
    assert lift.rendered([]) == "Every pin stands at its newest release."


@mark.parametrize(
    ("result", "found", "warned"),
    [
        param(
            {"stdout": dumps([[{"number": 1}], [{"number": 2}]])},
            [{"number": 1}, {"number": 2}],
            "",
            id = "pages"
        ),
        param(
            {"returncode": 1, "stderr": "HTTP 403\n"},
            [],
            "Dependabot alerts were not read: HTTP 403\n",
            id = "denied"
        )
    ]
)
def test_alerts_reads_every_page_or_warns(
    fp          : FakeProcess,
    found       : list[dict],
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    result      : dict,
    warned      : str
):
    """
    Pins that the open alerts on every page `gh` slurps come back as one
    list, and that an alert listing `gh` cannot read warns and returns
    none rather than stopping the table. The `stderr` the script binds on
    loading is replaced by a buffer, since pytest closes the stream `capsys`
    installs for setup once setup ends.
    """
    monkeypatch.setattr(lift, "stderr", StringIO())
    fp.register(
        [
            "gh", "api", "--paginate",
            "--slurp", "repos/{owner}/{repo}/dependabot/alerts?state=open&per_page=100"
        ],
        **result
    )

    assert lift.alerts() == found
    assert lift.stderr.getvalue() == warned


@mark.parametrize("argv", [["--try"], ["--write"], ["--try", "--write"]])
def test_main_moves_nothing_where_every_lift_is_held(
    argv        : list[str],
    lift        : ModuleType,
    monkeypatch : MonkeyPatch
):
    """
    Pins that a trial or a write holding back every lift the table lists
    runs no attempt and writes nothing, `mise relock` included.
    """
    ran = []
    monkeypatch.setattr(
        lift,
        "table",
        lambda: (
            [row(
                current = "1",
                lift    = lift,
                name    = "vite",
                newest  = "2"
            )],
            []
        )
    )
    monkeypatch.setattr(lift, "tried", lambda *args: ran.append("tried"))
    monkeypatch.setattr(lift, "write", lambda *args: ran.append("write"))
    monkeypatch.setattr("sys.argv", ["lift", *argv, "--hold", "vite"])

    lift.main()

    assert ran == []


@mark.parametrize(
    ("reported", "tags", "outcome"),
    [
        param({}, {}, None, id = "current"),
        param(
            {("cargo", "ruff-python-ast", "0.0.5"): "0.0.17"},
            {"0.16.10": "other"},
            "no ruff release tags ruff_python_ast@0.0.17, the commit 0.0.17 came from",
            id = "untagged"
        )
    ]
)
def test_ruff_takes_no_row_at_its_newest_crate_and_stops_on_an_untagged_one(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    outcome     : str | None,
    reported    : dict,
    tags        : dict[str, str]
):
    """
    Pins that `ruff_*` crates at their newest release take no row, and that
    a newest crate whose commit no ruff release tags stops the task rather
    than writing a release into `[package.metadata.ruff]` that names another
    commit.
    """
    pins = [
        lift.Pin(None, "ruff", "ruff_python_ast", "cargo", "0.0.5"),
        lift.Pin(None, "ruff", "ruff", "", "0.15.22")
    ]
    monkeypatch.setattr(
        lift,
        "published_commit",
        lambda crate, version: f"{crate}@{version}"
    )
    monkeypatch.setattr(lift, "tagged", lambda repo: tags)

    if outcome is None:
        assert lift.Lift.of("ruff", pins, lift.Reports(reported)) is None
    else:
        with raises(SystemExit) as stopped:
            lift.Lift.of("ruff", pins, lift.Reports(reported))
        assert stopped.value.code == outcome


@mark.parametrize(
    ("body", "encoding"),
    [param(compress(b"{}"), "gzip", id = "gzipped"), param(b"{}", None, id = "plain")]
)
def test_downloaded_names_the_task_and_undoes_a_gzip_encoding(
    body        : bytes,
    encoding    : str | None,
    lift        : ModuleType,
    monkeypatch : MonkeyPatch
):
    """
    Pins that every download sends the user agent crates.io asks API clients
    for and offers gzip, decompressing a gzipped body and passing a plain
    one through.
    """
    asked = []

    class Response(BytesIO):
        """
        Stands in for the response `urlopen` returns.
        """

        headers = {"Content-Encoding": encoding} if encoding else {}

    def opened(request: object, timeout: int) -> Response:
        """
        Records the request and returns the stand-in response.
        """
        asked.append((request.full_url, request.get_header("User-agent"), timeout))
        return Response(body)

    monkeypatch.setattr(lift, "urlopen", opened)

    assert lift.fetched("https://example.test/doc") == {}
    assert asked == [("https://example.test/doc", lift.AGENT, 120)]


def test_reports_read_the_newest_release_each_manager_prints(
    fp   : FakeProcess,
    lift : ModuleType
):
    """
    Pins the newest release read off each manager's report, meaning bun's
    `Latest` column, the release Cargo updates to or names as available
    with its version's build metadata dropped from the key, every tool
    `mise outdated` lists past its requested version, and uv's `latest`
    annotation, a script's tree included.
    """
    tree     = ["uv", "tree", "--depth", "1", "--outdated"]
    outdated = {
        "bun": {"latest": "1.4.3", "requested": "1.4.2"},
        "cargo:cargo-llvm-lines"   : {"latest": "0.4.48", "requested": "0.4.48"},
        "github:sharkdp/hyperfine" : {"latest": "2.0.0", "requested": "1.20.0"}
    }
    fp.register(
        ["bun", "outdated"],
        stdout = (
            "|---------------------------------------|\n"
            "| Package    | Current | Update | Latest |\n"
            "| vite (dev) | 8.3.0   | 8.3.0  | 8.3.4  |\n"
        )
    )
    fp.register(
        ["cargo", "update", "--dry-run", "--verbose", "--workspace"],
        stdout = (
            "     Locking 0 packages to highest Rust 1.99.0 compatible versions\n"
            "    Updating insta v1.48.0 -> v1.49.0\n"
            "   Unchanged toml v1.1.6+spec-1.1.0 (available: v1.1.8+spec-1.1.0)\n"
            "   Unchanged ruff_python_ast v0.0.5 (available: v0.0.17)\n"
        )
    )
    fp.register(["mise", "outdated", "--bump", "--json"], stdout=dumps(outdated))
    fp.register(
        [*tree, "--all-groups", "--frozen", "--project", "crate"],
        stdout = "prose-formatter\n├── pytest v9.1.1 (group: test) (latest: v9.2.0)\n"
    )
    fp.register(
        [*tree, "--script", "a.py"],
        stdout = "Resolved 1 package in 5ms\nmarkupsafe v3.0.3 (latest: v3.0.4)\n"
    )

    assert lift.Reports.read(["a.py"]).releases == {
        ("bun", "vite", "8.3.0")              : "8.3.4",
        ("cargo", "insta", "1.48.0")          : "1.49.0",
        ("cargo", "ruff-python-ast", "0.0.5") : "0.0.17",
        ("cargo", "toml", "1.1.6")            : "1.1.8+spec-1.1.0",
        ("mise", "bun", "1.4.2")              : "1.4.3",
        ("mise", "github:sharkdp/hyperfine", "1.20.0"): "2.0.0",
        ("uv", "markupsafe", "3.0.3") : "3.0.4",
        ("uv", "pytest", "9.1.1")     : "9.2.0"
    }


def test_ruff_names_the_release_its_newest_crate_was_published_from(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    released    : list[tuple]
):
    """
    Pins that the `ruff` row moves its crates to their newest release
    and `[package.metadata.ruff]` to the tag on the commit that crate was
    published from, showing the release.
    """
    tags   = {"0.16.10": "other", "0.17.0": "ruff_python_ast@0.0.17"}
    newest = lift.Reports({("cargo", "ruff-python-ast", "0.0.5"): "0.0.17"})
    pins   = [
        lift.Pin(None, "ruff", "ruff_python_ast", "cargo", "0.0.5"),
        lift.Pin(None, "ruff", "ruff", "", "0.15.22")
    ]
    monkeypatch.setattr(
        lift,
        "published_commit",
        lambda crate, version: f"{crate}@{version}"
    )
    monkeypatch.setattr(lift, "tagged", lambda repo: tags)

    lifted = lift.Lift.of("ruff", pins, newest)

    assert (lifted.current, lifted.newest) == ("0.15.22", "0.17.0")
    assert lifted.targets == {"0.0.5": "0.0.17", "0.15.22": "0.17.0"}
    assert released == [("ruff_python_ast", "0.0.17")]


def test_shown_drops_build_metadata_and_cuts_to_its_parts(lift: ModuleType):
    """
    Asserts that a home writes a version without its build metadata, cut to
    the components its `parts` counts.
    """
    assert lift.Home("Cargo.toml").shown("1.1.8+spec-1.1.0") == "1.1.8"
    assert lift.Home("README.md", parts=2).shown("1.99.0") == "1.99"


@mark.parametrize(
    "text",
    [
        param("2026-10-01T00:00:00Z", id = "utc"),
        param("2026-10-01T02:00:00+02:00", id = "offset"),
        param("2026-10-01T00:00:00", id = "no zone")
    ]
)
def test_stamp_reads_every_time_as_utc(lift: ModuleType, text: str):
    """
    Pins that a time naming a zone, an offset, or none reads as the same
    instant in UTC, so the held table's clearing time is the hour it labels.
    """
    assert lift.stamp(text).isoformat() == "2026-10-01T00:00:00+00:00"


def test_table_holds_back_each_tool_past_the_newest_its_report_gives(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    released    : list[tuple]
):
    """
    Pins that the table asks each tool for the releases held back past the
    newest release its report gives, or past its written version where the
    report omits it, and lists them by name and then by version.
    """
    asked = []
    found = [
        pin(
            keys    = ("tools", "zizmor"),
            lift    = lift,
            name    = "zizmor",
            path    = ".mise/config.toml",
            source  = "mise",
            written = "1.31.0"
        ),
        pin(
            keys    = ("tools", "rust"),
            lift    = lift,
            name    = "rust",
            path    = ".mise/config.toml",
            source  = "mise",
            written = "1.98.1"
        )
    ]
    holding = {
        "rust"   : [("rust", "1.99.10"), ("rust", "1.99.2")],
        "zizmor" : [("zizmor", "1.32.0")]
    }

    def held(holder: object, newest: str) -> list:
        """
        Records the newest release the pin `holder` is held back past and
        returns the releases `holding` names for it.
        """
        asked.append((holder.name, newest))
        return [lift.Held(None, *each) for each in holding[holder.name]]

    monkeypatch.setattr(lift, "pins", lambda: found)
    monkeypatch.setattr(lift, "alerts", lambda: [])
    monkeypatch.setattr(lift.Pin, "held", held)
    monkeypatch.setattr(
        lift.Reports,
        "read",
        lambda scripts: lift.Reports({("mise", "rust", "1.98.1"): "1.99.0"})
    )

    lifts, waiting = lift.table()

    assert sorted(asked) == [("rust", "1.99.0"), ("zizmor", "1.31.0")]
    assert [(each.name, each.version) for each in waiting] == [
        ("rust", "1.99.2"),
        ("rust", "1.99.10"),
        ("zizmor", "1.32.0")
    ]
    assert [(each.name, each.newest) for each in lifts] == [("rust", "1.99.0")]


@mark.parametrize(
    ("reported", "rows", "read"),
    [
        param(
            {("bun", "vite", "8.3.0"): "8.3.4"},
            [("vite", (7,))],
            True,
            id = "behind"
        ),
        param({}, [], False, id = "current")
    ]
)
def test_table_groups_the_pins_and_reads_alerts_only_for_a_lift(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    read        : bool,
    released    : list[tuple],
    reported    : dict,
    rows        : list[tuple]
):
    """
    Pins that the table holds one row per name whose pins trail their newest
    release, each carrying the open alerts it clears, and that it reads no
    alert where nothing trails, sending each task script to the reports.
    """
    asked  = []
    alerts = [
        {
            "number"     : 7,
            "dependency" : {"package": {"ecosystem": "npm", "name": "vite"}},
            "security_vulnerability": {"first_patched_version": {"identifier": "8.3.1"}}
        }
    ]
    found = [
        pin(
            lift    = lift,
            name    = "vite",
            path    = "site/package.json",
            pattern = "x",
            source  = "bun",
            written = "8.3.0"
        ),
        pin(
            lift    = lift,
            name    = "vite",
            path    = "site/package.json",
            pattern = "y",
            source  = "bun",
            written = "8.3.0"
        ),
        pin(
            lift    = lift,
            name    = "pyyaml",
            path    = ".mise/tasks/repo/audit.py",
            pattern = "z",
            source  = "uv",
            written = "6.0.3"
        )
    ]
    monkeypatch.setattr(lift, "pins", lambda: found)
    monkeypatch.setattr(
        lift.Reports,
        "read",
        lambda scripts: asked.append(scripts) or lift.Reports(reported)
    )
    monkeypatch.setattr(lift, "alerts", lambda: asked.append("alerts") or alerts)

    lifts, held = lift.table()
    assert ([(each.name, each.alerts) for each in lifts], held) == (rows, [])
    assert asked == [[".mise/tasks/repo/audit.py"], *(["alerts"] if read else [])]


def test_tagged_reads_an_annotated_tag_as_the_commit_beneath_it(
    fp   : FakeProcess,
    lift : ModuleType
):
    """
    Pins that a lightweight tag maps to the commit it names and that an
    annotated tag maps to the commit `ls-remote` peels it to, whichever line
    comes first, since that commit is the one a workflow pins.
    """
    fp.register(
        ["git", "ls-remote", "--tags", "https://github.com/owner/action"],
        stdout = f"{NEW}\trefs/tags/v2^{{}}\n{OLD}\trefs/tags/v1\n{OLD}\trefs/tags/v2\n"
    )

    assert lift.tagged("owner/action") == {"v1": OLD, "v2": NEW}


def test_tried_stops_where_no_attempt_passes(
    lift        : ModuleType,
    monkeypatch : MonkeyPatch,
    tmp_path    : Path
):
    """
    Pins that a trial no attempt passes stops the task before anything is
    written.
    """
    monkeypatch.setattr(lift, "LOGS", tmp_path)
    monkeypatch.setattr(lift, "attempt", lambda attempted, log: False)

    with raises(SystemExit) as stopped:
        lift.tried(
            set(),
            [row(
                current = "0.15.22",
                lift    = lift,
                name    = "ruff",
                newest  = "0.17.0"
            )]
        )

    assert stopped.value.code == "no attempt passed, so nothing is written"


def test_waiting_lists_each_held_release_under_the_cutoff(lift: ModuleType):
    """
    Pins the sentence naming the cutoff above the held table, and each row
    beside the time its release clears, empty where no time is known.
    """
    held = [
        lift.Held(WHEN + timedelta(hours=24), "rust", "1.99.1"),
        lift.Held(None, "zizmor", "1.32.0")
    ]

    assert lift.waiting(held).splitlines() == [
        "The `minimum_release_age` cutoff holds these releases back from every "
        "write until each clears:",
        "",
        "| **Pin**  | **Release** | **Clears**           |",
        "|----------|-------------|----------------------|",
        "| `rust`   | `1.99.1`    | 2026-10-02 00:00 UTC |",
        "| `zizmor` | `1.32.0`    |                      |"
    ]


def test_write_moves_every_home_and_each_lifted_package(
    capsys : CaptureFixture[str],
    fp     : FakeProcess,
    lift   : ModuleType,
    tree   : Path
):
    """
    Pins that a write moves every home of each lift, keeping the aligned
    layout of each manifest, locks and installs the tools a lifted mise pin
    names, names each lifted crate and each lifted project requirement to
    its manager under the written toolset, and runs `mise relock`, leaving
    the virtualenv in place where the `python` pin stays.
    """
    cargo = ["mise", "x", "--", "cargo", "update", "--package", "serde@1.0.229"]
    uv    = [
        "mise", "x", "--", "uv", "lock",
        "--project", "crate", "--upgrade-package", "pytest"
    ]
    tools  = [["mise", "lock"], ["mise", "install"]]
    lifted = [
        row(
            current = "1.15.0",
            lift    = lift,
            name    = "maturin",
            newest  = "1.16.0",
            pins    = (
                pin(
                    keys    = ("tools", "maturin"),
                    lift    = lift,
                    name    = "maturin",
                    path    = ".mise/config.toml",
                    source  = "mise",
                    written = "1.15.0"
                ),
            )
        ),
        row(
            current = "1.0.229",
            lift    = lift,
            name    = "serde",
            newest  = "1.0.230",
            pins    = (pin(
                keys    = ("workspace", "dependencies", "serde"),
                lift    = lift,
                name    = "serde",
                path    = "Cargo.toml",
                source  = "cargo",
                written = "1.0.229"
            ),)
        ),
        row(
            current = "9.1.1",
            lift    = lift,
            name    = "pytest",
            newest  = "9.2.0",
            pins    = (pin(
                lift    = lift,
                name    = "pytest",
                source  = "uv",
                written = "9.1.1"
            ),)
        ),
        row(
            current = "8.3.0",
            lift    = lift,
            name    = "vite",
            newest  = "8.3.4",
            pins    = tuple(
                pin(
                    lift    = lift,
                    name    = "vite",
                    path    = "site/package.json",
                    pattern = each,
                    source  = "bun",
                    written = "8.3.0"
                )
                for each in [r'"vite"\s*:\s*"[^"]*"', r'"vite@8.3.0"']
            )
        )
    ]
    for command in [*tools, cargo, uv]:
        fp.register(command)
    fp.register(SYNC, stdout=" M Cargo.lock\n")
    (tree / "crate/.venv").mkdir(parents=True)

    lift.write(lifted)

    assert (tree / ".mise/config.toml").read_text() == (
        TREE[".mise/config.toml"].replace('"1.15.0"', '"1.16.0"')
    )
    assert (tree / "Cargo.toml").read_text() == (
        TREE["Cargo.toml"].replace('"1.0.229"', '"1.0.230"')
    )
    assert (tree / "site/package.json").read_text() == (
        TREE["site/package.json"].replace("8.3.0", "8.3.4")
    )
    assert [list(call) for call in fp.calls] == [*tools, cargo, uv, SYNC]
    assert capsys.readouterr().out == " M Cargo.lock\n"
    assert (tree / "crate/.venv").is_dir()


def test_write_removes_the_virtualenv_where_python_moves(
    fp   : FakeProcess,
    lift : ModuleType,
    tree : Path
):
    """
    Pins that a write lifting the `python` pin removes the virtualenv the
    mise config declares once the new Python is installed, so mise creates
    it again on that Python rather than keeping the one it replaced.
    """
    for command in [["mise", "lock"], ["mise", "install"], SYNC]:
        fp.register(command)
    (tree / "crate/.venv/bin").mkdir(parents=True)

    lift.write(
        [row(
            current = "3.14.6",
            lift    = lift,
            name    = "python",
            newest  = "3.15.0",
            pins    = (pin(
                lift    = lift,
                name    = "python",
                source  = "mise",
                written = "3.14.6"
            ),)
        )]
    )

    assert not (tree / "crate/.venv").exists()
    assert [list(call) for call in fp.calls] == [
        ["mise", "lock"],
        ["mise", "install"],
        SYNC
    ]


def test_write_runs_only_the_relock_for_an_npm_lift(
    fp   : FakeProcess,
    lift : ModuleType,
    tree : Path
):
    """
    Pins that a lift touching no mise pin, crate, or locked Python package
    runs `mise relock` alone, since bun re-resolves an exact pin there.
    """
    fp.register(SYNC)
    vite = pin(
        lift    = lift,
        name    = "vite",
        path    = "site/package.json",
        pattern = r'"vite"\s*:\s*"[^"]*"',
        source  = "bun",
        written = "8.3.0"
    )

    lift.write(
        [row(
            current = "8.3.0",
            lift    = lift,
            name    = "vite",
            newest  = "8.3.4",
            pins    = (vite,)
        )]
    )

    assert [list(call) for call in fp.calls] == [SYNC]
