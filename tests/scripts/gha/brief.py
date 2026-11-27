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


@mark.usefixtures("gate")
def test_a_row_summary_lists_every_task_its_report_records(
    capsys      : CaptureFixture[str],
    monkeypatch : MonkeyPatch,
    task        : Callable[[str], ModuleType],
    tmp_path    : Path
):
    """
    Pins that the row summary names each task the `mise run` report holds
    with its arguments, status, and duration, in the report's order, and
    that a task with no duration shows a dash.
    """
    report = tmp_path / "tasks.json"
    report.write_text(
        dumps(
            {
                "tasks": [
                    {
                        "name"        : name,
                        "args"        : args,
                        "status"      : status,
                        "duration_ms" : ms
                    }
                    for name, args, status, ms in [
                        ("rust:build", [], "failed", 1500),
                        ("site:bake", ["--fast"], "passed", 20),
                        ("wasm:pack", [], "not_run", None)
                    ]
                ]
            }
        )
    )
    monkeypatch.setenv("REPORT", str(report))
    monkeypatch.setenv("ROW", "🪃 Browser")

    task("gha/brief").Summary().tasks()

    assert capsys.readouterr().out == (
        "## 🪃 Browser\n\n"
        "| Task | Status | Duration |\n"
        "|---|---|---|\n"
        "| `rust:build` | ❌ failed | 1.5s |\n"
        "| `site:bake --fast` | ✅ passed | 0.0s |\n"
        "| `wasm:pack` | ➖ not run | — |\n"
    )


@mark.usefixtures("gate")
def test_a_row_that_wrote_no_report_renders_no_summary(
    capsys      : CaptureFixture[str],
    monkeypatch : MonkeyPatch,
    task        : Callable[[str], ModuleType],
    tmp_path    : Path
):
    """
    Pins that a row whose run ended before `mise run` wrote its report
    renders nothing, so the summary step never fails a row on its own.
    """
    monkeypatch.setenv("REPORT", str(tmp_path / "missing.json"))
    monkeypatch.setenv("ROW", "🪃 Browser")

    task("gha/brief").Summary().tasks()

    assert capsys.readouterr().out == ""


@mark.parametrize("workflow", ["ci", "deploy"])
def test_every_matrix_row_runs_through_a_report(workflow: str):
    """
    Asserts that each matrix row of `ci.yml` and `deploy.yml` runs its task
    with `-c --report-file`, so sibling tasks all report, and that a step
    after it lists them even once the task failed.
    """
    flow  = safe_load((ROOT / f".github/workflows/{workflow}.yml").read_text())
    steps = flow["jobs"]["check" if workflow == "ci" else "checks"]["steps"]
    run   = next(step for step in steps if "--report-file" in step.get("run", ""))
    after = steps[steps.index(run) + 1]
    assert run["run"].startswith('mise run -c --report-file "$REPORT" ')
    assert after["run"] == "mise run gha:brief tasks"
    assert after["if"] == "${{ !cancelled() }}"
