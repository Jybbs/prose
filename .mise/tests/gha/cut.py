"""
Pins the release `gha:cut` reads for the crate's version, the path it takes
from what the repository's release listing holds, and that a `gh` call that
fails stops the task before any draft is cut.
"""

from collections.abc   import Callable
from io                import StringIO
from json              import dumps
from pathlib           import Path
from pytest            import CaptureFixture, MonkeyPatch, fixture, mark, param, raises
from pytest_subprocess import FakeProcess
from types             import ModuleType

CREATE = [
    "gh", "release", "create", "1.2.3", "--repo",
    "owner/repo", "--target", "main", "--generate-notes", "--draft"
]
LISTING  = ["gh", "api", "--paginate", "--slurp", "repos/owner/repo/releases"]
MANIFEST = '[package]\nversion = "{}"\n'
PREVIOUS = ["git", "show", "HEAD~1:crate/Cargo.toml"]


@fixture
def cut(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Returns the loaded `gha:cut` script with the `stderr` it binds on
    loading replaced by a buffer the case reads, since pytest closes the
    stream `capsys` installs for setup once setup ends.
    """
    module        = task("gha/cut")
    module.stderr = StringIO()
    return module


@fixture
def tree(monkeypatch: MonkeyPatch, tmp_path: Path) -> Path:
    """
    Returns a working tree whose `crate/Cargo.toml` carries version `1.2.3`
    and makes it the current directory. The environment names `owner/repo`,
    no event, and no output file, so the outputs print to stdout.
    """
    (manifest := tmp_path / "crate/Cargo.toml").parent.mkdir()
    manifest.write_text(MANIFEST.format("1.2.3"), encoding="utf-8")
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("GITHUB_REPOSITORY", "owner/repo")
    monkeypatch.delenv("GITHUB_EVENT_NAME", raising=False)
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    return tmp_path


def release(tag: str, draft: bool = False) -> dict:
    """
    Returns the fields `gha:cut` reads from one entry of the release
    listing.
    """
    return {"draft": draft, "html_url": f"https://example.test/{tag}", "tag_name": tag}


@mark.parametrize(
    ("pages", "state", "url", "annotation"),
    [
        param(
            [[release("1.2.2")], []],
            "cut",
            "https://example.test/new",
            "",
            id = "absent"
        ),
        param(
            [[release("1.2.2")], [release("1.2.3", draft=True)]],
            "drafted",
            "https://example.test/1.2.3",
            "::notice::Draft for 1.2.3 already exists, leaving it untouched.\n",
            id = "draft"
        ),
        param(
            [[release("1.2.3"), release("1.2.2")]],
            "published",
            "https://example.test/1.2.3",
            "::warning::1.2.3 is already published, skipping the draft cut.\n",
            id = "published"
        ),
        param(
            [
                [release("1.2.3", draft=True)],
                [{**release("1.2.3"), "html_url": "https://example.test/older"}]
            ],
            "drafted",
            "https://example.test/1.2.3",
            "::notice::Draft for 1.2.3 already exists, leaving it untouched.\n",
            id = "first match"
        )
    ]
)
def test_main_branches_on_the_listing(
    annotation : str,
    capsys     : CaptureFixture[str],
    cut        : ModuleType,
    fp         : FakeProcess,
    pages      : list,
    state      : str,
    tree       : Path,
    url        : str
):
    """
    Pins that a moved version is looked up across every page of the listing,
    that a version no release carries is cut as a draft, and that a draft or
    a published release carrying it is reported and left as it stands.
    """
    fp.register(PREVIOUS, stdout=MANIFEST.format("1.2.2"))
    fp.register(LISTING, stdout=dumps(pages))
    fp.register(CREATE, stdout=f"{url}\n")

    cut.main()

    assert list(fp.calls) == [PREVIOUS, LISTING, *([CREATE] if state == "cut" else [])]
    assert capsys.readouterr().out == f"state={state}\nurl={url}\nversion=1.2.3\n"
    assert cut.stderr.getvalue() == annotation


def test_emit_appends_to_the_output_file(
    cut         : ModuleType,
    monkeypatch : MonkeyPatch,
    tmp_path    : Path
):
    """
    Asserts that `emit` appends one line per output to the file
    `$GITHUB_OUTPUT` names, after what an earlier step wrote there.
    """
    output = tmp_path / "output"
    output.write_text("earlier=1\n", encoding="utf-8")
    monkeypatch.setenv("GITHUB_OUTPUT", str(output))

    cut.emit(state="", version="1.2.3")

    assert output.read_text(encoding="utf-8") == "earlier=1\nstate=\nversion=1.2.3\n"


@mark.parametrize(
    ("listing", "calls"),
    [
        param(
            {"returncode": 1, "stderr": "gh: Bad credentials (HTTP 401)\n"},
            [PREVIOUS, LISTING],
            id = "listing"
        ),
        param({"stdout": dumps([[]])}, [PREVIOUS, LISTING, CREATE], id = "create")
    ]
)
def test_main_stops_on_the_gh_call_that_fails(
    calls   : list,
    capsys  : CaptureFixture[str],
    cut     : ModuleType,
    fp      : FakeProcess,
    listing : dict,
    tree    : Path
):
    """
    Pins that a `gh` call exiting nonzero stops the task before it emits
    any output, with an `::error::` annotation carrying what `gh` printed on
    stderr, so no draft is cut after a listing that fails.
    """
    fp.register(PREVIOUS, stdout=MANIFEST.format("1.2.2"))
    fp.register(LISTING, **listing)
    fp.register(CREATE, returncode=1, stderr="gh: Bad credentials (HTTP 401)\n")

    with raises(SystemExit) as stopped:
        cut.main()

    assert stopped.value.code == "::error::gh: Bad credentials (HTTP 401)"
    assert list(fp.calls) == calls
    assert capsys.readouterr().out == ""


@mark.parametrize(
    ("event", "calls", "state", "url"),
    [
        param("push", [PREVIOUS], "", "", id = "push"),
        param(
            "workflow_dispatch",
            [PREVIOUS, LISTING],
            "published",
            "https://example.test/1.2.3",
            id = "dispatch"
        )
    ]
)
def test_main_reads_the_listing_at_an_unchanged_version_only_on_a_dispatch(
    calls       : list,
    capsys      : CaptureFixture[str],
    cut         : ModuleType,
    event       : str,
    fp          : FakeProcess,
    monkeypatch : MonkeyPatch,
    state       : str,
    tree        : Path,
    url         : str
):
    """
    Pins that the task reads no release on a push whose version matches
    HEAD~1's and still reads the listing on a `workflow_dispatch`, emitting
    the version either way.
    """
    monkeypatch.setenv("GITHUB_EVENT_NAME", event)
    fp.register(PREVIOUS, stdout=MANIFEST.format("1.2.3"))
    fp.register(LISTING, stdout=dumps([[release("1.2.3")]]))

    cut.main()

    assert list(fp.calls) == calls
    assert capsys.readouterr().out == f"state={state}\nurl={url}\nversion=1.2.3\n"


@mark.parametrize(
    ("stdout", "returncode", "version"),
    [
        param(MANIFEST.format("1.2.2"), 0, "1.2.2", id = "manifest"),
        param("", 128, None, id = "absent"),
        param('[package]\nname = "prose"\n', 0, None, id = "no version"),
        param("[package\n", 0, None, id = "malformed")
    ]
)
def test_previous_version_reads_the_manifest_at_the_parent_commit(
    cut        : ModuleType,
    fp         : FakeProcess,
    returncode : int,
    stdout     : str,
    version    : str | None
):
    """
    Pins that the version at HEAD~1 is read from its `crate/Cargo.toml`, and
    that a missing manifest, a manifest with no version, and one that does
    not parse each read as no version.
    """
    fp.register(PREVIOUS, returncode=returncode, stdout=stdout)

    assert cut.previous_version() == version
