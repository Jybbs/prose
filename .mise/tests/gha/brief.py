"""
Pins the Deploy gate `gha:brief` renders, reading the jobs it judges from
the `gate` job's `needs` in `deploy.yml`, so a row the workflow waits on and
the verdict leaves out fails here.
"""

from collections.abc import Callable
from json            import dumps
from pathlib         import Path
from pytest          import CaptureFixture, MonkeyPatch, fixture, mark, raises
from types           import ModuleType
from yaml            import safe_load

ROOT = Path(__file__).resolve().parents[3]
JOBS = safe_load((ROOT / ".github/workflows/deploy.yml").read_text())["jobs"]
URL  = "https://preview.example.test"


@fixture
def gate(
    capsys      : CaptureFixture[str],
    monkeypatch : MonkeyPatch,
    task        : Callable[[str], ModuleType]
) -> Callable[..., tuple[int, str]]:
    """
    Returns a function running the Deploy gate with each job the workflow's
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
