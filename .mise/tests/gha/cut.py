"""
Pins the release `gha:cut` reads for the crate's version, the outcome it
takes from what the repository's release listing holds (creating the draft,
refreshing it, or failing on a published version), and that a `gh` call that
fails stops the task before any draft is written.
"""

from collections.abc   import Callable
from json              import dumps, loads
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
NOTES    = [
    "gh", "api", "--method", "POST", "repos/owner/repo/releases/generate-notes",
    "-f", "tag_name=1.2.3", "-f", "target_commitish=main"
]
UPDATE = [
    "gh", "api", "--method", "PATCH", "repos/owner/repo/releases/7", "--input", "-"
]


@fixture
def cut(task: Callable[[str], ModuleType]) -> ModuleType:
    """
    Loads the `gha:cut` script.
    """
    return task("gha/cut")


@fixture
def tree(monkeypatch: MonkeyPatch, workdir: Callable[[dict[str, str]], Path]) -> Path:
    """
    Writes a working tree whose `crate/Cargo.toml` carries version `1.2.3`
    and makes it the current directory. The environment names `owner/repo`
    and no output file, so the outputs print to stdout.
    """
    monkeypatch.setenv("GITHUB_REPOSITORY", "owner/repo")
    monkeypatch.delenv("GITHUB_OUTPUT", raising=False)
    return workdir({"crate/Cargo.toml": MANIFEST.format("1.2.3")})


def release(tag: str, draft: bool = False) -> dict:
    """
    Builds the fields `gha:cut` reads from one entry of the release listing.
    """
    return {
        "draft"    : draft,
        "html_url" : f"https://example.test/{tag}",
        "id"       : 7,
        "tag_name" : tag
    }


@mark.parametrize(
    ("pages", "calls", "state", "url"),
    [
        param(
            [[release("1.2.2")], []],
            [LISTING, CREATE],
            "created",
            "https://example.test/new",
            id = "absent"
        ),
        param(
            [[release("1.2.2")], [release("1.2.3", draft=True)]],
            [LISTING, NOTES, UPDATE],
            "refreshed",
            "https://example.test/1.2.3",
            id = "draft"
        ),
        param(
            [
                [release("1.2.3", draft=True)],
                [{
                    **release("1.2.3"),
                    "html_url" : "https://example.test/older",
                    "id"       : 9
                }]
            ],
            [LISTING, NOTES, UPDATE],
            "refreshed",
            "https://example.test/1.2.3",
            id = "first match"
        )
    ]
)
def test_main_creates_or_refreshes_the_draft(
    calls  : list,
    capsys : CaptureFixture[str],
    cut    : ModuleType,
    fp     : FakeProcess,
    pages  : list,
    state  : str,
    tree   : Path,
    url    : str
):
    """
    Pins that the version is looked up across every page of the listing,
    that a version no release carries is created as a draft, and that a
    draft carrying it has its body replaced with the generated notes alone.
    """
    fp.register(LISTING, stdout=dumps(pages))
    fp.register(CREATE, stdout=f"{url}\n")
    fp.register(NOTES, stdout=dumps({"body": "## Notes\n", "name": "1.2.3"}))
    fp.register(UPDATE)

    cut.main()

    assert list(fp.calls) == calls
    assert capsys.readouterr().out == f"version=1.2.3\nstate={state}\nurl={url}\n"


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


def test_main_fails_on_a_published_version(
    capsys : CaptureFixture[str],
    cut    : ModuleType,
    fp     : FakeProcess,
    tree   : Path
):
    """
    Pins that a published release carrying the version stops the task with
    an `::error::` annotation naming `mise bump`, after it emits the outputs
    the summary links the release from, and writes no release.
    """
    fp.register(LISTING, stdout=dumps([[release("1.2.3"), release("1.2.2")]]))

    with raises(SystemExit) as stopped:
        cut.main()

    assert stopped.value.code == (
        "::error::1.2.3 is already published. Run `mise bump` to move "
        "crate/Cargo.toml to the next version."
    )
    assert list(fp.calls) == [LISTING]
    assert capsys.readouterr().out == (
        "version=1.2.3\nstate=published\nurl=https://example.test/1.2.3\n"
    )


def test_main_sends_the_generated_notes_as_the_whole_body(
    cut  : ModuleType,
    fp   : FakeProcess,
    tree : Path
):
    """
    Asserts that the refresh sends the body the generate-notes endpoint
    returns and no other field, so the draft's title stays as it stands.
    """
    sent: list[bytes] = []
    fp.register(LISTING, stdout=dumps([[release("1.2.3", draft=True)]]))
    fp.register(NOTES, stdout=dumps({"body": "## Notes\n", "name": "1.2.3"}))
    fp.register(UPDATE, stdin_callable=sent.append)

    cut.main()

    assert [loads(chunk) for chunk in sent] == [{"body": "## Notes\n"}]


@mark.parametrize(
    ("listing", "calls"),
    [
        param(
            {"returncode": 1, "stderr": "gh: Bad credentials (HTTP 401)\n"},
            [LISTING],
            id = "listing"
        ),
        param({"stdout": dumps([[]])}, [LISTING, CREATE], id = "create"),
        param(
            {"stdout": dumps([[release("1.2.3", draft=True)]])},
            [LISTING, NOTES],
            id = "notes"
        )
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
    Pins that a `gh` call exiting nonzero stops the task with an `::error::`
    annotation carrying what `gh` printed on stderr, having emitted the
    version alone, so no draft is written after a read that fails.
    """
    fp.register(LISTING, **listing)
    fp.register(CREATE, returncode=1, stderr="gh: Bad credentials (HTTP 401)\n")
    fp.register(NOTES, returncode=1, stderr="gh: Bad credentials (HTTP 401)\n")

    with raises(SystemExit) as stopped:
        cut.main()

    assert stopped.value.code == "::error::gh: Bad credentials (HTTP 401)"
    assert list(fp.calls) == calls
    assert capsys.readouterr().out == "version=1.2.3\n"


@mark.parametrize(
    ("printed", "annotation"),
    [
        param(
            "HTTP 422: Validation Failed "
            "(https://api.github.com/repos/owner/repo/releases)\n"
            "Release.tag_name already exists\n",
            "::error::HTTP 422: Validation Failed "
            "(https://api.github.com/repos/owner/repo/releases)"
            "%0ARelease.tag_name already exists",
            id = "newline"
        ),
        param(
            "first\r\nsecond\n",
            "::error::first%0D%0Asecond",
            id = "carriage return"
        ),
        param("100% of %0A\n", "::error::100%25 of %250A", id = "percent")
    ]
)
def test_gh_raises_every_line_of_its_stderr_in_one_annotation(
    annotation : str,
    cut        : ModuleType,
    fp         : FakeProcess,
    printed    : str
):
    """
    Pins that a failed `gh` call stops the task with one `::error::` line
    carrying all of its stderr. Each `%`, carriage return, and newline is
    escaped the way `@actions/core` escapes a command's message, so the
    runner restores every line in the annotation, a `%0A` that `gh` printed
    included.
    """
    fp.register(CREATE, returncode=1, stderr=printed)

    with raises(SystemExit) as stopped:
        cut.gh(*CREATE[1:])

    assert stopped.value.code == annotation
