"""
Pins the Deploy and Warm gates `gha:brief` renders, reading the jobs each
judges from the `gate` job's `needs` in `deploy.yml` and `warm.yml`, so a
row the workflow waits on and the verdict leaves out fails here, the line
the Draft summary renders for each outcome `gha:cut` emits, and the workflow
token the Deploy site build reads GitHub with.
"""

from collections.abc import Callable
from json            import dumps
from pathlib         import Path
from pytest          import CaptureFixture, MonkeyPatch, fixture, mark, param, raises
from types           import ModuleType
from yaml            import safe_load

ROOT   = Path(__file__).resolve().parents[3]
COMMIT = "[`abcdef0`](https://example.test/owner/repo/commit/abcdef012345)"
CUT    = safe_load((ROOT / ".github/workflows/draft.yml").read_text())["jobs"]["cut"]
DRAFT  = "https://example.test/owner/repo/releases/tag/untagged-1"
JOBS   = safe_load((ROOT / ".github/workflows/deploy.yml").read_text())["jobs"]
URL    = "https://preview.example.test"
WARM   = safe_load((ROOT / ".github/workflows/warm.yml").read_text())["jobs"]


@fixture
def gate(
    capsys      : CaptureFixture[str],
    monkeypatch : MonkeyPatch,
    task        : Callable[[str], ModuleType]
) -> Callable[..., tuple[int, str]]:
    """
    Builds a function running the Deploy gate with each job the workflow's
    gate needs passing unless named, and returning the exit code and the
    summary it printed.
    """
    monkeypatch.chdir(ROOT)
    monkeypatch.delenv("GITHUB_STEP_SUMMARY", raising=False)
    monkeypatch.setenv("GITHUB_REPOSITORY", "owner/repo")
    monkeypatch.setenv("GITHUB_SERVER_URL", "https://example.test")
    monkeypatch.setenv("REF", "main")
    monkeypatch.setenv("SHA", "abcdef012345")
    brief = task("gha/brief")

    def run(**results: str) -> tuple[int, str]:
        """
        Runs the gate over `results`, a job's result by its name.
        """
        needs = {job: {
            "result": results.get(job, "success")
        } for job in JOBS["gate"]["needs"]}
        if needs["deploy"]["result"] == "success":
            needs["deploy"]["outputs"] = {"url": URL}
        monkeypatch.setenv("NEEDS", dumps(needs))
        with raises(SystemExit) as stopped:
            brief.Summary().deploy()
        return int(stopped.value.code), capsys.readouterr().out

    return run


@mark.parametrize(
    ("job", "stage"),
    [("fit", "Squeeze"), ("render", "Render"), ("sweep", "Axe")]
)
def test_a_failed_browser_row_names_its_stage(
    gate  : Callable[..., tuple[int, str]],
    job   : str,
    stage : str
):
    """
    Pins that a failed browser row skips the publish and that the summary
    names that row rather than the job it skipped.
    """
    code, summary = gate(**{"deploy": "skipped", job: "failure"})
    assert code == 1
    assert f"{stage} failed" in summary


@mark.parametrize("job", ["fit", "render", "sweep"])
def test_a_failed_browser_row_withholds_the_publish(job: str):
    """
    Asserts that the publish job waits on each browser row, so a failure in
    one publishes nothing.
    """
    assert job in JOBS["deploy"]["needs"]


@mark.parametrize("job", JOBS["gate"]["needs"])
def test_a_failed_job_fails_the_gate(gate: Callable[..., tuple[int, str]], job: str):
    """
    Asserts that each job the gate waits on fails it alone.
    """
    assert gate(**{job: "failure"})[0] == 1


def test_every_job_passing_passes_the_gate(gate: Callable[..., tuple[int, str]]):
    """
    Asserts that a run whose every row passed exits 0 on the live preview.
    """
    code, summary = gate()
    assert code == 0
    assert URL in summary


@mark.parametrize("job", WARM["gate"]["needs"])
def test_a_failed_job_fails_the_warm_gate(
    gate        : Callable[..., tuple[int, str]],
    job         : str,
    monkeypatch : MonkeyPatch,
    task        : Callable[[str], ModuleType]
):
    """
    Asserts that each job the Warm gate waits on fails it alone, reading the
    jobs from the `gate` job's `needs` in `warm.yml`.
    """
    needs = {need: {
        "result": "failure" if need == job else "success"
    } for need in WARM["gate"]["needs"]}
    monkeypatch.setenv("NEEDS", dumps(needs))
    with raises(SystemExit) as stopped:
        task("gha/brief").Summary().warm()
    assert stopped.value.code == 1


@mark.parametrize(
    ("state", "line"),
    [
        param(
            "created",
            f"Created the [*Prose* 1.2.3]({DRAFT}) draft from {COMMIT}.",
            id = "created"
        ),
        param(
            "refreshed",
            f"Refreshed the notes of the [*Prose* 1.2.3]({DRAFT}) draft from {COMMIT}.",
            id = "refreshed"
        ),
        param(
            "published",
            f"[*Prose* 1.2.3]({DRAFT}) is already published. Run `mise bump` to move "
            "`crate/Cargo.toml` to the next version.",
            id = "published"
        ),
        param("", f"No draft for *Prose* 1.2.3 was written from {COMMIT}.", id = "none")
    ]
)
@mark.usefixtures("gate")
def test_the_draft_summary_names_each_outcome(
    capsys      : CaptureFixture[str],
    line        : str,
    monkeypatch : MonkeyPatch,
    state       : str,
    task        : Callable[[str], ModuleType]
):
    """
    Pins the line the Draft summary renders for each state `gha:cut` emits,
    linking the draft or the published release, and for a run that stopped
    before it emitted one.
    """
    monkeypatch.setenv("DRAFT_STATE", state)
    monkeypatch.setenv("DRAFT_URL", DRAFT)
    monkeypatch.setenv("VERSION", "1.2.3")

    task("gha/brief").Summary().draft()

    assert capsys.readouterr().out == f"## 🪻 Prose Draft\n\n{line}\n"


def test_the_draft_summary_runs_after_a_failed_cut():
    """
    Asserts that the Draft summary step runs unless the run was cancelled,
    so a cut that failed still renders its line.
    """
    steps = {step["name"]: step for step in CUT["steps"]}
    assert steps["🗞️ Brief"]["if"] == "${{ !cancelled() }}"


def test_the_site_build_reads_github_with_the_workflow_token():
    """
    Asserts that the Deploy step building the site passes the workflow token
    as `GITHUB_TOKEN`, so its GitHub reads run under the token's rate limit.
    """
    steps = {step["name"]: step for step in JOBS["press"]["steps"]}
    assert steps["Build the site and check internal links"]["env"] == {
        "GITHUB_TOKEN": "${{ github.token }}"
    }
