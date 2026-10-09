"""
Pins the calls `repo:rulesets` sends to GitHub, the settings file it reads
them from, and what it rejects before sending any.
"""

from dataclasses import FrozenInstanceError, fields, is_dataclass
from json        import dumps, loads
from pathlib     import Path
from pytest      import MonkeyPatch, fixture, mark, param, raises
from tomllib     import loads as from_toml

ROOT = Path(__file__).resolve().parents[3]

PROJECT = """
[project]
description = "Describes the repository."
keywords    = ["alpha", "beta"]

[project.urls]
homepage = "https://example.test:8443/docs"
issues   = "https://example.test/issues"
"""

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

RULESETS = "repos/{owner}/{repo}/rulesets"
BASE     = "repos/{owner}/{repo}"

SENT = [
    (
        "PATCH", BASE,
        {
            "allow_auto_merge"            : True,
            "allow_merge_commit"          : False,
            "allow_rebase_merge"          : True,
            "allow_squash_merge"          : True,
            "allow_update_branch"         : False,
            "delete_branch_on_merge"      : False,
            "description"                 : "Describes the repository.",
            "has_discussions"             : True,
            "has_issues"                  : False,
            "has_projects"                : True,
            "has_wiki"                    : True,
            "homepage"                    : "example.test",
            "security_and_analysis"       : {
                "secret_scanning"                 : {"status": "disabled"},
                "secret_scanning_push_protection" : {"status": "enabled"}
            },
            "squash_merge_commit_message" : "COMMIT_MESSAGES",
            "squash_merge_commit_title"   : "COMMIT_OR_PR_TITLE",
            "web_commit_signoff_required" : True
        }
    ),
    ("PUT", f"{BASE}/topics", {"names": ["alpha", "beta"]}),
    ("DELETE", f"{BASE}/vulnerability-alerts", None),
    ("PUT", f"{BASE}/automated-security-fixes", None),
    ("DELETE", f"{BASE}/private-vulnerability-reporting", None),
    (
        "PUT", f"{BASE}/actions/permissions",
        {"allowed_actions": "selected", "enabled": True, "sha_pinning_required": False}
    ),
    (
        "PUT", f"{BASE}/actions/permissions/workflow",
        {
            "can_approve_pull_request_reviews" : True,
            "default_workflow_permissions"     : "write"
        }
    )
]


@fixture
def rulesets(task):
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
        (target := tmp_path / name).parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    return tmp_path


def argv(method: str, endpoint: str, body: dict | None) -> list[str]:
    """
    Returns the command `gh` is run with for a call.
    """
    return [
        "gh", "api", "--method", method, endpoint,
        *(["--input", "-"] if body else [])
    ]


def parsed(rulesets, text: str = SETTINGS):
    """
    Returns the record the settings `text` parses to.
    """
    return rulesets.record(rulesets.Settings, from_toml(text))


@mark.parametrize(
    ("old", "new", "message"),
    [
        param(
            "[actions]\n", "[actions]\nextra = 1\n",
            r"\[actions\] declares unknown keys \['extra'\]",
            id = "unknown key"
        ),
        param(
            "[workflow]\n", "[extra]\n\n[workflow]\n",
            r"\.github/settings.toml declares unknown keys \['extra'\]",
            id = "unknown table"
        ),
        param(
            "[repository.security_and_analysis.secret_scanning]\n",
            "[repository.security_and_analysis.secret_scanning]\nextra = 1\n",
            r"\[repository.security_and_analysis.secret_scanning\] declares unknown keys \['extra'\]",
            id = "unknown nested key"
        ),
        param(
            'enabled              = true\n', "",
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
            'default_workflow_permissions     = "write"', "default_workflow_permissions = 1",
            r"\[workflow\] key 'default_workflow_permissions' is not a str",
            id = "wrong type"
        ),
        param(
            "has_wiki                    = true", 'has_wiki = "true"',
            r"\[repository\] key 'has_wiki' is not a bool",
            id = "string for a bool"
        ),
        param(
            "private-vulnerability-reporting", "private_vulnerability_reporting",
            r"\[advisories\] declares unknown keys",
            id = "underscore in an endpoint table"
        ),
        param(
            "has_wiki ", "has-wiki ",
            r"\[repository\] declares unknown keys \['has-wiki'\]",
            id = "hyphen in a repository key"
        )
    ]
)
def test_record_rejects_a_malformed_file(rulesets, old: str, new: str, message: str):
    """
    Pins that an unknown or missing table or key, a value of the wrong type,
    and the wrong spelling of a key each stop the read, naming where.
    """
    assert old in SETTINGS
    with raises(SystemExit, match=message):
        parsed(rulesets, SETTINGS.replace(old, new))


def test_record_rejects_a_value_that_is_not_a_table(rulesets):
    """
    Pins that a key holding a scalar where a table belongs stops the read,
    naming the table.
    """
    with raises(SystemExit, match=r"\[workflow\] is not a table"):
        rulesets.record(rulesets.Settings, {**from_toml(SETTINGS), "workflow": 1})


def test_record_accepts_the_tracked_file(rulesets):
    """
    Asserts that `.github/settings.toml` itself reads into the record.
    """
    parsed(rulesets, (ROOT / ".github/settings.toml").read_text(encoding="utf-8"))


def test_requests_send_every_setting_in_order(rulesets):
    """
    Pins the endpoint, method, and body of every call, the repository `PATCH`
    carrying the description and the homepage's bare domain, the topics
    following it, and the dependabot, advisory, Actions, and workflow calls
    after.
    """
    assert parsed(rulesets).requests(from_toml(PROJECT)["project"]) == [
        (endpoint, method, body) for method, endpoint, body in SENT
    ]


def test_tracked_files_send_the_about_box(rulesets):
    """
    Pins that the tracked `crate/pyproject.toml` and `.github/settings.toml`
    send the description, the homepage's bare domain, and the topics the
    repository carries.
    """
    project  = from_toml((ROOT / "crate/pyproject.toml").read_text(encoding="utf-8"))
    tracked  = (ROOT / ".github/settings.toml").read_text(encoding="utf-8")
    settings = parsed(rulesets, tracked)
    (_, _, patch), (_, _, topics) = settings.requests(project["project"])[:2]
    assert patch["description"] == "A Python typesetter for the reader."
    assert patch["homepage"] == "prose.fyi"
    assert topics["names"] == [
        "alignment", "code-quality", "developer-tools", "formatter", "linter",
        "python", "rust", "static-analysis", "typesetting"
    ]


def records(value):
    """
    Yields a record and every record nested beneath it.
    """
    yield value
    for field in fields(value):
        if is_dataclass(child := getattr(value, field.name)):
            yield from records(child)


def test_record_is_frozen(rulesets):
    """
    Pins that every record the settings file reads into rejects assignment.
    """
    for record in records(parsed(rulesets)):
        with raises(FrozenInstanceError):
            setattr(record, next(iter(vars(record))), None)


@mark.parametrize("on", [True, False])
def test_requests_toggle_an_endpoint_by_method(rulesets, on: bool):
    """
    Pins that a `true` flag in an endpoint table sends `PUT` and a `false`
    one sends `DELETE`, neither with a body.
    """
    text = SETTINGS.replace(
        "private-vulnerability-reporting = false",
        f"private-vulnerability-reporting = {str(on).lower()}"
    )
    project  = from_toml(PROJECT)["project"]
    advisory = parsed(rulesets, text).requests(project)[4]
    assert advisory == (
        f"{BASE}/private-vulnerability-reporting", "PUT" if on else "DELETE", None
    )


def test_main_applies_the_rulesets_then_every_setting(
    fp, rulesets, tree: Path, capsys
):
    """
    Pins the whole run, that a ruleset is replaced where a live one has its
    name and created where none does, that every setting follows in order with
    its body on standard input, and that a live ruleset no file names is
    listed.
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
            argv(method, endpoint, body),
            stdout         = dumps(live) if method == "GET" else "",
            stdin_callable = lambda data: sent.append(loads(data))
        )

    rulesets.main()

    assert list(fp.calls) == [argv(*call) for call in expected]
    assert sent == [body for _, _, body in expected if body]
    assert capsys.readouterr().out.splitlines()[-3:] == [
        "repository settings applied from .github/settings.toml",
        "rulesets on GitHub that .github/rulesets omits:",
        "  stray"
    ]


def test_main_sends_nothing_when_the_settings_are_malformed(fp, rulesets, tree: Path):
    """
    Asserts that a settings file the record rejects stops the run before any
    `gh` call, `fp` raising on a command nothing registered.
    """
    (tree / ".github/settings.toml").write_text(
        SETTINGS + "[extra]\n", encoding="utf-8"
    )
    with raises(SystemExit, match="declares unknown keys"):
        rulesets.main()
    assert not fp.calls


def test_main_sends_nothing_when_the_project_omits_the_homepage(
    fp,
    rulesets,
    tree: Path
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
