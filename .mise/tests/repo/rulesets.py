"""
Pins the calls `repo:rulesets` sends to GitHub, the settings file it reads
them from, and what it rejects before sending any.
"""

from collections.abc   import Callable, Iterator
from dataclasses       import FrozenInstanceError
from functools         import reduce
from json              import dumps, loads
from pathlib           import Path
from pytest            import CaptureFixture, MonkeyPatch, fixture, mark, param, raises
from pytest_subprocess import FakeProcess
from tomllib           import loads as from_toml
from types             import ModuleType
from typing            import Any

ROOT    = Path(__file__).resolve().parents[3]
BASE    = "repos/{owner}/{repo}"
PROJECT = """
[project]
description = "Describes the repository."
keywords    = ["alpha", "beta"]

[project.urls]
homepage = "https://example.test:8443/docs"
issues   = "https://example.test/issues"
"""
RULESETS = "repos/{owner}/{repo}/rulesets"
SETTINGS = """
[actions]
allowed_actions      = "selected"
enabled              = true
sha_pinning_required = false

[advisories]
private-vulnerability-reporting = false

[dependabot]
automated-security-fixes = true
vulnerability-alerts     = false

[repository]
allow_auto_merge            = true
allow_merge_commit          = false
allow_rebase_merge          = true
allow_squash_merge          = true
allow_update_branch         = false
delete_branch_on_merge      = false
has_discussions             = true
has_issues                  = false
has_projects                = true
has_wiki                    = true
squash_merge_commit_message = "COMMIT_MESSAGES"
squash_merge_commit_title   = "COMMIT_OR_PR_TITLE"
web_commit_signoff_required = true

[repository.security_and_analysis.secret_scanning]
status = "disabled"

[repository.security_and_analysis.secret_scanning_push_protection]
status = "enabled"

[workflow]
can_approve_pull_request_reviews = true
default_workflow_permissions     = "write"
"""
SENT = [
    (
        "PATCH", BASE,
        {
            "allow_auto_merge"       : True,
            "allow_merge_commit"     : False,
            "allow_rebase_merge"     : True,
            "allow_squash_merge"     : True,
            "allow_update_branch"    : False,
            "delete_branch_on_merge" : False,
            "description"            : "Describes the repository.",
            "has_discussions"        : True,
            "has_issues"             : False,
            "has_projects"           : True,
            "has_wiki"               : True,
            "homepage"               : "example.test",
            "squash_merge_commit_message" : "COMMIT_MESSAGES",
            "squash_merge_commit_title"   : "COMMIT_OR_PR_TITLE",
            "web_commit_signoff_required" : True,
            "security_and_analysis"       : {
                "secret_scanning"                 : {"status": "disabled"},
                "secret_scanning_push_protection" : {"status": "enabled"}
            }
        }
    ),
    ("PUT", f"{BASE}/topics", {"names": ["alpha", "beta"]}),
    ("DELETE", f"{BASE}/vulnerability-alerts", None),
    ("PUT", f"{BASE}/automated-security-fixes", None),
    ("DELETE", f"{BASE}/private-vulnerability-reporting", None),
    (
        "PUT",
        f"{BASE}/actions/permissions",
        {"allowed_actions": "selected", "enabled": True, "sha_pinning_required": False}
    ),
    (
        "PUT",
        f"{BASE}/actions/permissions/workflow",
        {
            "can_approve_pull_request_reviews" : True,
            "default_workflow_permissions"     : "write"
        }
    )
]


@fixture
def rulesets(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Returns the loaded `repo:rulesets` script.
    """
    return task("repo/rulesets")


@fixture
def tree(monkeypatch: MonkeyPatch, tmp_path: Path) -> Path:
    """
    Returns a working tree holding the sample settings, the sample project
    table, and two rulesets, one named `kept` and one named `fresh`, and
    makes it the current directory.
    """
    for name, text in {
        ".github/rulesets/a.json" : dumps({"name": "kept"}),
        ".github/rulesets/b.json" : dumps({"name": "fresh"}),
        ".github/settings.toml"   : SETTINGS,
        "crate/pyproject.toml"    : PROJECT
    }.items():
        (target := tmp_path / name).parent.mkdir(exist_ok=True, parents=True)
        target.write_text(text, encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    return tmp_path


def argv(body: dict | None, endpoint: str, method: str) -> list[str]:
    """
    Returns the command `gh` is run with for a call.
    """
    return [
        "gh", "api", "--method", method, endpoint,
        *(["--input", "-"] if body else [])
    ]


def parsed(rulesets: ModuleType, text: str = SETTINGS) -> Any:
    """
    Returns the record the settings `text` parses to.
    """
    return rulesets.record(rulesets.Settings, from_toml(text))


def tables(table: dict, keys: tuple[str, ...] = ()) -> Iterator[tuple[str, ...]]:
    """
    Yields the keys leading to `table` and to every table nested beneath it.
    """
    yield keys
    for key, value in table.items():
        if isinstance(value, dict):
            yield from tables(value, (*keys, key))


@mark.parametrize(
    ("old", "new", "message"),
    [
        param(
            "[actions]\n",
            "[actions]\nextra = 1\n",
            r"\[actions\] declares unknown keys \['extra'\]",
            id = "unknown key"
        ),
        param(
            "[workflow]\n",
            "[extra]\n\n[workflow]\n",
            r"\.github/settings.toml declares unknown keys \['extra'\]",
            id = "unknown table"
        ),
        param(
            "[repository.security_and_analysis.secret_scanning]\n",
            "[repository.security_and_analysis.secret_scanning]\nextra = 1\n",
            (
                r"\[repository.security_and_analysis.secret_scanning\] declares "
                r"unknown keys \['extra'\]"
            ),
            id = "unknown nested key"
        ),
        param(
            "enabled              = true\n",
            "",
            r"\[actions\] omits keys \['enabled'\]",
            id = "missing key"
        ),
        param(
            "[workflow]\ncan_approve_pull_request_reviews = true\n"
            'default_workflow_permissions     = "write"\n',
            "",
            r"\.github/settings.toml omits keys \['workflow'\]",
            id = "missing table"
        ),
        param(
            'default_workflow_permissions     = "write"',
            "default_workflow_permissions = 1",
            r"\[workflow\] key 'default_workflow_permissions' is not a str",
            id = "wrong type"
        ),
        param(
            "has_wiki                    = true",
            'has_wiki = "true"',
            r"\[repository\] key 'has_wiki' is not a bool",
            id = "string for a bool"
        ),
        param(
            "private-vulnerability-reporting",
            "private_vulnerability_reporting",
            r"\[advisories\] declares unknown keys",
            id = "underscore in an endpoint table"
        ),
        param(
            "has_wiki ",
            "has-wiki ",
            r"\[repository\] declares unknown keys \['has-wiki'\]",
            id = "hyphen in a repository key"
        )
    ]
)
def test_record_rejects_a_malformed_file(
    rulesets : ModuleType,
    old      : str,
    new      : str,
    message  : str
):
    """
    Pins that an unknown or missing table or key, a value of the wrong type,
    and the wrong spelling of a key each stop the read, naming where.
    """
    assert old in SETTINGS
    with raises(SystemExit, match=message):
        parsed(rulesets, SETTINGS.replace(old, new))


def test_main_applies_the_rulesets_then_every_setting(
    capsys   : CaptureFixture[str],
    fp       : FakeProcess,
    rulesets : ModuleType,
    tree     : Path
):
    """
    Pins the whole run, that a ruleset is replaced where a live one has its
    name and created where none does, that every setting follows in order
    with its body on standard input, and that a live ruleset no file names
    is listed.
    """
    expected = [
        ("GET", RULESETS, None),
        ("PUT", f"{RULESETS}/7", {"name": "kept"}),
        ("POST", RULESETS, {"name": "fresh"}),
        *SENT
    ]
    live = [{"id": 7, "name": "kept"}, {"id": 8, "name": "stray"}]
    sent = []
    for method, endpoint, body in expected:
        fp.register(
            argv(body, endpoint, method),
            stdout         = dumps(live) if method == "GET" else "",
            stdin_callable = lambda data: sent.append(loads(data))
        )

    rulesets.main()

    assert list(fp.calls) == [
        argv(body, endpoint, method) for method, endpoint, body in expected
    ]
    assert sent == [body for _, _, body in expected if body]
    assert capsys.readouterr().out.splitlines()[-3:] == [
        "repository settings applied from .github/settings.toml",
        "rulesets on GitHub that .github/rulesets omits:",
        "  stray"
    ]


def test_main_sends_nothing_when_the_project_omits_the_homepage(
    fp       : FakeProcess,
    rulesets : ModuleType,
    tree     : Path
):
    """
    Asserts that a project table with no `homepage` URL stops the run before
    any `gh` call, the rulesets included.
    """
    (tree / "crate/pyproject.toml").write_text(
        PROJECT.replace('homepage = "https://example.test:8443/docs"\n', ""),
        encoding = "utf-8"
    )
    with raises(KeyError, match="homepage"):
        rulesets.main()
    assert not fp.calls


def test_main_sends_nothing_when_the_settings_are_malformed(
    fp       : FakeProcess,
    rulesets : ModuleType,
    tree     : Path
):
    """
    Asserts that a settings file the record rejects stops the run before any
    `gh` call, `fp` raising on a command nothing registered.
    """
    (tree / ".github/settings.toml").write_text(
        SETTINGS + "[extra]\n",
        encoding = "utf-8"
    )
    with raises(SystemExit, match="declares unknown keys"):
        rulesets.main()
    assert not fp.calls


def test_record_accepts_the_tracked_file(rulesets: ModuleType):
    """
    Asserts that `.github/settings.toml` itself reads into the record.
    """
    parsed(rulesets, tracked(".github/settings.toml"))


def test_record_rejects_a_value_that_is_not_a_table(rulesets: ModuleType):
    """
    Pins that a key holding a scalar where a table belongs stops the read,
    naming the table.
    """
    with raises(SystemExit, match=r"\[workflow\] is not a table"):
        rulesets.record(rulesets.Settings, {**from_toml(SETTINGS), "workflow": 1})


@mark.parametrize(
    "keys",
    list(tables(from_toml(SETTINGS))),
    ids = lambda keys: ".".join(keys) or "settings"
)
def test_record_is_frozen(keys: tuple[str, ...], rulesets: ModuleType):
    """
    Pins that the record each table of the sample settings reads into
    rejects assignment.
    """
    record = reduce(getattr, keys, parsed(rulesets))
    with raises(FrozenInstanceError):
        setattr(record, next(iter(vars(record))), None)


@mark.parametrize("on", [True, False])
def test_requests_toggle_an_endpoint_by_method(rulesets: ModuleType, on: bool):
    """
    Pins that a `true` flag in an endpoint table sends `PUT` and a `false`
    one sends `DELETE`, neither with a body.
    """
    text = SETTINGS.replace(
        "private-vulnerability-reporting = false",
        f"private-vulnerability-reporting = {str(on).lower()}"
    )
    assert parsed(rulesets, text).requests(from_toml(PROJECT)["project"])[4] == (
        f"{BASE}/private-vulnerability-reporting",
        "PUT" if on else "DELETE",
        None
    )


def test_requests_send_every_setting_in_order(rulesets: ModuleType):
    """
    Pins the endpoint, method, and body of every call, the repository
    `PATCH` carrying the description and the homepage's bare domain, the
    topics following it, and the dependabot, advisory, Actions, and workflow
    calls after.
    """
    assert parsed(rulesets).requests(from_toml(PROJECT)["project"]) == [
        (endpoint, method, body) for method, endpoint, body in SENT
    ]


def test_tracked_files_send_the_about_box(rulesets: ModuleType):
    """
    Pins that the tracked `crate/pyproject.toml` and `.github/settings.toml`
    send the project's description, the bare domain of its homepage, and its
    keywords as the topics.
    """
    project  = from_toml(tracked("crate/pyproject.toml"))["project"]
    settings = parsed(rulesets, tracked(".github/settings.toml"))
    (_, _, patch), (_, _, topics) = settings.requests(project)[:2]
    assert patch["description"] == project["description"]
    assert f"https://{patch['homepage']}" == project["urls"]["homepage"]
    assert topics["names"] == project["keywords"]


def tracked(path: str) -> str:
    """
    Returns the text of the tracked file at `path` under the worktree root.
    """
    return (ROOT / path).read_text(encoding="utf-8")
