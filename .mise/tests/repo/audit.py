"""
Pins how `repo:audit` reads a GitHub `paths` filter, the `sources` patterns
it finds a filter leaving out across a task and the tasks it depends on, and
the trigger it reports for each. Also pins the label `parity` reports a pin
under once either side of its pair drifts, and the annotation a `fail` in a
helper of `parity` stops the audit on.
"""

from collections.abc   import Callable
from json              import dumps
from pathlib           import Path
from pytest            import CaptureFixture, MonkeyPatch, fixture, mark, param, raises
from pytest_subprocess import FakeProcess
from subprocess        import run
from types             import ModuleType

COVERING = ["Cargo.lock", "site/**", "wasm/**"]
OMITTING = ["Cargo.lock", "site/**"]
WHEEL    = ".github/actions/build-wheel/action.yml"
DRIFTS   = {
    ".mise/config.toml maturin pin ↔ build-wheel action maturin-version": [
        (".mise/config.toml", "1.16.0", "1.15.0"),
        (WHEEL, "v1.16.0", "v1.15.0")
    ],

    ".mise/config.toml maturin pin ↔ crate/pyproject.toml build requirement": [
        (".mise/config.toml", "1.16.0", "1.15.0"),
        ("crate/pyproject.toml", "maturin==1.16.0", "maturin==1.15.0")
    ],

    ".mise/config.toml ruff pin ↔ crate/Cargo.toml ruff release": [
        (".mise/config.toml", "0.17.3", "0.17.2"),
        ("crate/Cargo.toml", "0.17.3", "0.17.2")
    ],

    ".mise/config.toml wasm-bindgen pin ↔ Cargo.lock wasm-bindgen": [
        (".mise/config.toml", "0.2.130", "0.2.129"),
        ("Cargo.lock", "0.2.130", "0.2.129")
    ],

    "Cargo.toml members ↔ .github/workflows/warm.yml path filter": [
        ("Cargo.toml", ', "crate"', ""),
        (".github/workflows/warm.yml", "      - crate/Cargo.toml\n", "")
    ],

    "Cargo.toml members ↔ lock:sync sources": [
        ("Cargo.toml", ', "crate"', ""),
        ("lock:sync", "crate/Cargo.toml", "other/Cargo.toml")
    ],

    "Cargo.toml rust-version ↔ .mise/config.toml rust pin": [
        ("Cargo.toml", "1.99.1", "1.98.1"),
        (".mise/config.toml", "1.99.1", "1.98.1")
    ],

    "README.md Python badge ↔ crate/pyproject.toml requires-python": [
        (".github/README.md", "3.10+", "3.11+"),
        ("crate/pyproject.toml", ">=3.10", ">=3.11")
    ],

    "README.md Rust badge ↔ Cargo.toml rust-version": [
        (".github/README.md", "1.99+", "1.98+"),
        ("Cargo.toml", "1.99.1", "1.98.1")
    ],

    "actions/cache/save in .github/workflows/ci.yml ↔ .github/workflows/warm.yml": [
        (".github/workflows/ci.yml", "save@2222", "save@2223"),
        (".github/workflows/warm.yml", "save@2222", "save@2223")
    ],

    "lock:check sources ↔ lock:sync sources and outputs": [
        ("lock:check", "Cargo.lock", "uv.lock"),
        ("lock:sync", ".mise/config.toml", ".mise/other.toml"),
        ("lock:sync", "Cargo.lock", "uv.lock")
    ],

    "site/package.json @shikijs/magic-move patch key ↔ its devDependency pin": [
        ("site/package.json", "magic-move@4.6.0", "magic-move@4.5.0"),
        ("site/package.json", 'magic-move": "4.6.0', 'magic-move": "4.5.0')
    ],

    "site/package.json vite override ↔ its devDependency pin": [
        ("site/package.json", '8.3.5"}, "patched', '8.3.4"}, "patched'),
        ("site/package.json", '8.3.5"}, "overrides', '8.3.4"}, "overrides')
    ],

    "site:test sources ↔ site:press sources": [
        ("site:test", '"**/*"', '"lib/**/*"'),
        ("site:press", "site/**/*", "site/lib/**/*")
    ],

    "wasm:pack sources ↔ restore-wasm hashFiles paths": [
        ("wasm:pack", "wasm/**", "wasm/src/**"),
        (".github/actions/restore-wasm/action.yml", "wasm/**", "wasm/src/**")
    ],

    f"actions/checkout in {WHEEL} ↔ .github/workflows/ci.yml": [
        (WHEEL, "checkout@1111", "checkout@1112"),
        (".github/workflows/ci.yml", "checkout@1111", "checkout@1112")
    ],

    f"actions/checkout in {WHEEL} ↔ .github/workflows/warm.yml": [
        (WHEEL, "checkout@1111", "checkout@1112"),
        (".github/workflows/warm.yml", "checkout@1111", "checkout@1112")
    ]
}
PINS = {
    ".github/README.md": (
        "[![Rust](rust.svg)![1.99+](https://img.shields.io/badge/1.99+-8a80cb)]\n"
        "[![Python](python.svg)![3.10+](https://img.shields.io/badge/3.10+-8a80cb)]\n"
    ),

    ".github/actions/restore-wasm/action.yml":
        "key : ${{ hashFiles('wasm/**', 'Cargo.toml') }}\n",

    ".github/workflows/ci.yml": (
        "steps:\n"
        "  - uses : actions/checkout@1111\n"
        "  - uses : &cache-save actions/cache/save@2222\n"
    ),

    ".github/workflows/warm.yml": (
        "on:\n"
        "  push:\n"
        "    paths:\n"
        "      - Cargo.toml\n"
        "      - wasm/Cargo.toml\n"
        "      - crate/Cargo.toml\n"
        "jobs:\n"
        "  warm:\n"
        "    steps:\n"
        "      - uses : actions/checkout@1111\n"
        "      - uses : &cache-save actions/cache/save@2222\n"
    ),

    ".mise/config.toml": (
        "[tools]\n"
        '"github:wasm-bindgen/wasm-bindgen" = { version = "0.2.130", os = ["linux"] }\n'
        'maturin = "1.16.0"\n'
        'ruff = "0.17.3"\n'
        'rust = { version = "1.99.1", profile = "minimal" }\n'
    ),

    "Cargo.lock": (
        '[[package]]\nname = "anyhow"\nversion = "1.0.0"\n\n'
        '[[package]]\nname = "wasm-bindgen"\nversion = "0.2.130"\n'
    ),

    "Cargo.toml": (
        '[workspace]\nmembers = ["wasm", "crate"]\n\n'
        '[workspace.package]\nrust-version = "1.99.1"\n'
    ),

    "crate/Cargo.toml": '[package.metadata.ruff]\nrelease = "0.17.3"\n',

    "crate/pyproject.toml": (
        '[build-system]\nrequires = ["setuptools", "maturin==1.16.0"]\n\n'
        '[project]\nrequires-python = ">=3.10"\n'
    ),

    WHEEL: (
        "steps:\n"
        "  - uses : actions/checkout@1111\n"
        "  - uses : PyO3/maturin-action@3333\n"
        "    with:\n"
        "      maturin-version : v1.16.0\n"
    ),

    "site/package.json": dumps(
        {
            "devDependencies"     : {"@shikijs/magic-move": "4.6.0", "vite": "8.3.5"},
            "overrides"           : {"postcss": "8.5.32", "vite": "8.3.5"},
            "patchedDependencies" : {"@shikijs/magic-move@4.6.0": "magic-move.patch"}
        }
    )
}
REPORT = "::error::{}.yml {} paths omit sources {} reads: ['wasm/**']\n"
TASKS  = [
    {
        "dir"     : "{root}",
        "name"    : "lock:check",
        "depends" : [],
        "sources" : [
            "Cargo.lock", "{root}/.mise/config.toml", "{root}/Cargo.lock",
            "{root}/crate/Cargo.toml", "{root}/wasm/Cargo.toml"
        ]
    },
    {
        "dir"     : "{root}",
        "name"    : "lock:sync",
        "depends" : [],
        "outputs" : ["Cargo.lock"],
        "sources" : [
            "{root}/.mise/config.toml", "{root}/crate/Cargo.toml",
            "{root}/wasm/Cargo.toml"
        ]
    },
    {
        "dir"     : "{root}",
        "name"    : "rust:build",
        "depends" : [],
        "sources" : []
    },
    {
        "dir"     : "{root}/site",
        "name"    : "site:bake",
        "depends" : [],
        "sources" : ["{root}/Cargo.lock"]
    },
    {
        "dir"     : "{root}/site",
        "name"    : "site:press",
        "depends" : ["rust:build", "site:bake", ["wasm:pack", "--release"]],
        "sources" : ["{root}/site/**/*", "!{root}/site/dist/**/*"]
    },
    {
        "dir"     : "{root}/site",
        "name"    : "site:test",
        "depends" : [],
        "sources" : ["**/*", "!dist/**/*"]
    },
    {
        "dir"     : "{root}",
        "name"    : "wasm:pack",
        "depends" : [],
        "sources" : ["{root}/Cargo.toml", "{root}/wasm/**"]
    }
]
TREE = {
    "Cargo.lock"       : "",
    "site/dist/app.js" : "",
    "site/index.md"    : "",
    "site/lib/a.ts"    : "",
    "wasm/lib.rs"      : ""
}


@fixture
def audit(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Returns the loaded `repo:audit` script.
    """
    return task("repo/audit")


@fixture
def drifted(
    fp      : FakeProcess,
    workdir : Callable[[dict[str, str]], Path]
) -> Callable[[str, str, str], None]:
    """
    Returns a function making the current directory a tree holding `PINS`,
    with `mise` faked to list `TASKS`, after one edit to a pin.
    """
    def write(home: str, pin: str, stray: str):
        """
        Writes the tree and registers the listing with `pin` replaced by
        `stray` in the file or the task `home` names, failing where `home`
        holds no `pin`.
        """
        homes = PINS | {task["name"]: dumps(task) for task in TASKS}
        assert pin in homes[home]
        homes[home] = homes[home].replace(pin, stray)
        listing     = ",".join(homes.pop(task["name"]) for task in TASKS)
        fp.register(
            ["mise", "tasks", "ls", "--json"],
            stdout = f"[{listing}]".replace("{root}", str(workdir(homes)))
        )

    return write


@fixture
def tree(
    fp          : FakeProcess,
    monkeypatch : MonkeyPatch,
    workdir     : Callable[[dict[str, str]], Path]
):
    """
    Makes the current directory a git repository tracking `TREE`, with
    `mise` faked to list `TASKS` and one config file declaring a tool, and
    every `git` call run as given.
    """
    for variable in ("GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"):
        monkeypatch.delenv(variable, raising=False)
    fp.keep_last_process(True)
    fp.pass_command(["git", fp.any()])
    root = workdir(TREE)
    run(["git", "init", "-q"], check=True)
    run(["git", "add", "."], check=True)
    fp.register(
        ["mise", "config", "ls", "--json"],
        stdout = dumps([{"path": f"{root}/.mise/config.toml", "tools": ["uv"]}])
    )
    fp.register(
        ["mise", "tasks", "ls", "--json"],
        stdout = dumps(TASKS).replace("{root}", str(root))
    )


@mark.parametrize(
    ("patterns", "path", "selected"),
    [
        param(["*"], "README.md", True, id="star-matches-a-root-file"),
        param(["*"], "docs/README.md", False, id="star-stops-at-a-slash"),
        param(["*.js"], "appxjs", False, id="dot-reads-literally"),
        param(["*.jsx?"], "page.js", True, id="question-mark-takes-none"),
        param(["*.jsx?"], "page.jsx", True, id="question-mark-takes-one"),
        param(["*.jsx?"], "page.jsxx", False, id="question-mark-takes-no-second"),
        param(["*.jsx?"], "page.jsy", False, id="question-mark-other-character"),
        param(["**"], "all/the/files.md", True, id="doublestar-crosses-slashes"),
        param(["**.js"], "index.js", True, id="doublestar-suffix-root"),
        param(["**.js"], "src/js/app.js", True, id="doublestar-suffix-nested"),
        param(["docs/*"], "docs/file.txt", True, id="directory-star-matches"),
        param(["docs/*"], "docs/mona/octocat.txt", False, id="directory-star-stops"),
        param(["docs/**"], "docs/mona/octocat.txt", True, id="directory-doublestar"),
        param(["docs/**/*.md"], "docs/README.md", True, id="slash-no-directory"),
        param(["docs/**/*.md"], "docs/a/markdown/file.md", True, id="slash-nested"),
        param(["**/docs/**"], "docs/hello.md", True, id="leading-root"),
        param(["**/docs/**"], "space/docs/plan/space.doc", True, id="leading-nested"),
        param(["**/README.md"], "README.md", True, id="named-file-root"),
        param(["**/README.md"], "js/README.md", True, id="named-file-nested"),
        param(["**/*src/**"], "my-src/code/js/app.js", True, id="directory-suffix"),
        param(["**/*-post.md"], "path/their-post.md", True, id="file-suffix"),
        param(["**/migrate-*.sql"], "db/sept/migrate-v1.sql", True, id="file-prefix"),
        param(["v[12].[0-9]+.[0-9]+"], "v1.10.1", True, id="plus-takes-several"),
        param(["v[12].[0-9]+.[0-9]+"], "v2.0.0", True, id="plus-takes-one"),
        param(["v[12].[0-9]+.[0-9]+"], "v1..0", False, id="plus-takes-at-least-one"),
        param(["v[12].[0-9]+.[0-9]+"], "v3.0.0", False, id="brackets-unlisted-digit"),
        param(["[CB]at"], "Bat", True, id="brackets-listed-letter"),
        param(["[CB]at"], "Rat", False, id="brackets-unlisted-letter"),
        param(["[1-2]00"], "200", True, id="brackets-range-end"),
        param(["[1-2]00"], "300", False, id="brackets-outside-a-range"),
        param(["[0-9a-z]"], "5", True, id="brackets-inside-a-digit-range"),
        param(["[0-9a-z]"], "q", True, id="brackets-inside-a-letter-range"),
        param(["[0-9a-z]"], "Q", False, id="brackets-outside-both-ranges"),
        param(["*.md", "!README.md"], "hello.md", True, id="negation-other-file"),
        param(["*.md", "!README.md"], "README.md", False, id="negation-excludes"),
        param(["*.md", "!README.md"], "docs/hello.md", False, id="negation-nested"),
        param(["*.md", "!README.md", "README*"], "README.md", True, id="reincluded"),
        param(["*.md", "!README.md", "README*"], "README.doc", True, id="later-match")
    ]
)
def test_filtered_reads_github_globs(
    audit    : ModuleType,
    path     : str,
    patterns : list[str],
    selected : bool
):
    """
    Pins each wildcard and each path row of GitHub's filter pattern cheat
    sheet, the last pattern to match a path deciding.
    """
    assert audit.filtered(path, patterns) is selected


@mark.parametrize(
    ("omitted", "paths"),
    [
        param(set(), COVERING, id="covered"),
        param({"Cargo.lock"}, ["site/**", "wasm/**"], id="string-dependency-omitted"),
        param({"wasm/**"}, OMITTING, id="list-dependency-omitted"),
        param(
            {"site/**/*"},
            ["Cargo.lock", "site/lib/**", "wasm/**"],
            id = "top-level-file-omitted"
        ),
        param(
            set(),
            ["Cargo.lock", "site/index.md", "site/lib/**", "wasm/**"],
            id = "excluded-file-ignored"
        )
    ]
)
@mark.usefixtures("tree")
def test_unfiltered_reports_each_omitted_source(
    audit   : ModuleType,
    omitted : set[str],
    paths   : list[str]
):
    """
    Pins that a source pattern selecting a tracked file the filter misses is
    reported, for a task and for either form of a dependency, whereas a file
    a `!` source excludes is not.
    """
    assert audit.unfiltered(paths, "site:press") == omitted


@mark.parametrize(
    ("module", "pull_request", "push", "reported"),
    [
        param(COVERING, COVERING, COVERING, "", id="covered"),
        param(
            COVERING,
            OMITTING,
            COVERING,
            REPORT.format("deploy", "pull_request", "site:press"),
            id = "deploy-pull-request"
        ),
        param(
            COVERING,
            COVERING,
            OMITTING,
            REPORT.format("deploy", "push", "site:press"),
            id = "deploy-push"
        ),
        param(
            OMITTING,
            COVERING,
            COVERING,
            REPORT.format("module", "push", "wasm:pack"),
            id = "module-push"
        )
    ]
)
@mark.usefixtures("tree")
def test_triggers_reports_each_trigger_omitting_a_source(
    audit        : ModuleType,
    capsys       : CaptureFixture[str],
    module       : list[str],
    pull_request : list[str],
    push         : list[str],
    reported     : str,
    workdir      : Callable[[dict[str, str]], Path]
):
    """
    Pins that both `deploy.yml` triggers are held to the sources of
    `site:press` and the `module.yml` push trigger to those of `wasm:pack`,
    the audit failing on an annotation naming the trigger and the pattern.
    """
    workdir(
        {
            ".github/workflows/deploy.yml" : dumps(
                {
                    "on": {
                        "pull_request" : {"paths": pull_request},
                        "push"         : {"paths": push}
                    }
                }
            ),
            ".github/workflows/module.yml" : dumps({"on": {"push": {"paths": module}}})
        }
    )

    assert audit.triggers() == bool(reported)
    assert capsys.readouterr().out == reported


@mark.parametrize(
    "flow",
    [
        param("on:\n  pull_request:\n    paths: ['**']\n", id="event-absent"),
        param("on:\n  push:\n", id="event-bare"),
        param("on:\n  push:\n    branches: [main]\n", id="paths-absent")
    ]
)
def test_workflow_paths_fails_without_a_filter(
    audit   : ModuleType,
    capsys  : CaptureFixture[str],
    flow    : str,
    workdir : Callable[[dict[str, str]], Path]
):
    """
    Pins that a trigger declaring no `paths` stops the audit on an
    annotation naming the event and the workflow.
    """
    workdir({".github/workflows/deploy.yml": flow})

    with raises(SystemExit, match="1"):
        audit.workflow_paths("push", "deploy")

    assert capsys.readouterr().out == "::error::no push paths filter in deploy.yml\n"


@mark.parametrize(
    ("label", "home", "pin", "stray"),
    [(label, *side) for label, sides in DRIFTS.items() for side in sides]
)
def test_parity_reports_each_drifted_side_under_its_label(
    audit   : ModuleType,
    capsys  : CaptureFixture[str],
    drifted : Callable[[str, str, str], None],
    home    : str,
    label   : str,
    pin     : str,
    stray   : str
):
    """
    Pins that drifting either side of a pair, written out or generated,
    fails `parity` on an annotation naming that pair's label.
    """
    drifted(home, pin, stray)

    assert audit.parity() == 1
    assert f"::error::parity mismatch in {label}: " in capsys.readouterr().out


@mark.parametrize(
    ("home", "pin", "stray"),
    [
        param("Cargo.toml", "crate", "crate", id="nothing-drifted"),
        param(
            WHEEL,
            "maturin-action@3333",
            "maturin-action@3334",
            id = "action-pinned-once"
        ),
        param(
            "site/package.json",
            '"postcss": "8.5.32"',
            '"postcss": "8.5.31"',
            id = "override-of-a-transitive"
        ),
        param("site/package.json", '"overrides"', '"resolutions"', id="no-overrides"),
        param(
            "site/package.json",
            '"patchedDependencies"',
            '"patches"',
            id = "no-patched-dependencies"
        )
    ]
)
def test_parity_passes_where_no_pair_differs(
    audit   : ModuleType,
    capsys  : CaptureFixture[str],
    drifted : Callable[[str, str, str], None],
    home    : str,
    pin     : str,
    stray   : str
):
    """
    Pins that `parity` reports nothing for a tree whose pairs agree, leaving
    alone an action pinned in one home and an override of a package the
    manifest does not pin.
    """
    drifted(home, pin, stray)

    assert audit.parity() == 0
    assert capsys.readouterr().out == ""


def test_parity_annotates_both_sides_of_a_drifted_pair(
    audit   : ModuleType,
    capsys  : CaptureFixture[str],
    drifted : Callable[[str, str, str], None]
):
    """
    Pins that the annotation for a drifted pair holds the value read from
    each side, and that a pin only one pair reads fails that pair alone.
    """
    drifted(".mise/config.toml", "0.17.3", "0.17.2")

    assert audit.parity() == 1
    assert capsys.readouterr().out == (
        "::error::parity mismatch in .mise/config.toml ruff pin ↔ crate/Cargo.toml "
        "ruff release: '0.17.2' vs '0.17.3'\n"
    )


def test_parity_pairs_each_have_a_drift_row(
    audit   : ModuleType,
    drifted : Callable[[str, str, str], None]
):
    """
    Pins that the labels of the pairs `parity` compares over `PINS` are
    exactly the keys of `DRIFTS`, so a pair the audit adds over `PINS` fails
    this case until `DRIFTS` holds a row for it.
    """
    drifted("Cargo.toml", "crate", "crate")

    assert {label for label, _, _ in audit.parity_pairs()} == set(DRIFTS)


@mark.parametrize(
    ("home", "pin", "stray", "reported"),
    [
        param(
            ".github/README.md",
            "rust.svg",
            "ruby.svg",
            "no README.md badge line carries 'rust.svg'",
            id = "badge"
        ),
        param(
            WHEEL,
            "maturin-version",
            "maturin-release",
            "no maturin-version pin in build-wheel/action.yml",
            id = "action-input"
        ),
        param(
            "crate/pyproject.toml",
            "maturin==",
            "maturin>=",
            "maturin build requirement is not an exact pin",
            id = "inexact-build-requirement"
        ),
        param(
            "crate/pyproject.toml",
            "maturin==",
            "pdm==",
            "no build-system requirement named 'maturin'",
            id = "absent-build-requirement"
        ),
        param(
            "Cargo.lock",
            '"wasm-bindgen"',
            '"wasm-pack"',
            "no Cargo.lock package named 'wasm-bindgen'",
            id = "lock-package"
        ),
        param(
            ".github/actions/restore-wasm/action.yml",
            "hashFiles",
            "hashGlobs",
            "no hashFiles call in restore-wasm/action.yml",
            id = "hash-call"
        ),
        param(
            "lock:sync",
            '"outputs": ["Cargo.lock"]',
            '"outputs": {"auto": true}',
            "no outputs declaration in the lock:sync task",
            id = "task-key"
        ),
        param(
            "Cargo.toml",
            "1.99.1",
            "stable",
            "cannot parse major.minor from 'stable'",
            id = "unversioned"
        ),
        param(
            "site/package.json",
            "magic-move@",
            "magic-fade@",
            "no site/package.json devDependency named '@shikijs/magic-fade'",
            id = "patch-key"
        )
    ]
)
def test_parity_stops_on_a_side_holding_no_pin(
    audit    : ModuleType,
    capsys   : CaptureFixture[str],
    drifted  : Callable[[str, str, str], None],
    home     : str,
    pin      : str,
    reported : str,
    stray    : str
):
    """
    Pins that a `fail` in a helper of `parity` stops the audit on one
    annotation naming what is missing.
    """
    drifted(home, pin, stray)

    with raises(SystemExit, match="1"):
        audit.parity()

    assert capsys.readouterr().out == f"::error::{reported}\n"
