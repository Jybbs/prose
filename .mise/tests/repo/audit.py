"""
Pins how `repo:audit` reads a GitHub `paths` filter, the `sources` patterns
it finds a filter leaving out across a task and the tasks it depends on, and
the trigger it reports for each.
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
REPORT   = "::error::{}.yml {} paths omit sources {} reads: ['wasm/**']\n"
TASKS    = [
    {
        "dir"     : "{root}",
        "name"    : "lock:sync",
        "depends" : [],
        "sources" : ["{root}/.mise/config.toml"]
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
        "depends" : ["site:bake", ["wasm:pack", "--release"]],
        "sources" : ["{root}/site/**/*", "!{root}/site/dist/**/*"]
    },
    {
        "dir"     : "{root}",
        "name"    : "wasm:pack",
        "depends" : [],
        "sources" : ["{root}/wasm/**"]
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
