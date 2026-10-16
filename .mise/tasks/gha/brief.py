#!/usr/bin/env -S uv run --script
# MISE description="Render a workflow step summary and exit with its gate verdict"
# /// script
# requires-python = ">=3.11"
# dependencies = ["jinja2==3.1.6", "markupsafe==3.0.4", "pyyaml==6.0.3"]
# ///
"""
Renders a Prose step summary and gates the workflow's exit code.

Every gate but `draft` passes `NEEDS`, which is the workflow's `needs`
context as JSON. A job's result is `NEEDS[job]["result"]` and its outputs
sit under `NEEDS[job]["outputs"]`.

Subcommands:
    ci      : Renders the CI gate summary. Reads the `check` result plus the
              GitHub-runner defaults. Exits 0 when that matrix succeeded.
    corpus  : Renders the Corpus gate summary. Reads the result of every job
              the corpus workflow's gate depends on, plus the GitHub-runner
              defaults. Exits 0 when every job that ran succeeded, the delta
              reporting without gating.
    deploy  : Renders the Deploy gate summary. Reads the `checks`, `press`,
              and `deploy` results, the `deploy` job's `url` output, and the
              GitHub-runner defaults. Exits 0 when all three succeeded.
    draft   : Renders the Draft summary. Reads `DRAFT_STATE`, `DRAFT_URL`,
              and `VERSION`, plus the GitHub-runner defaults.
    release : Renders the Release gate summary. Reads the `build`, `sdist`,
              `validate`, and `publish` results plus the GitHub-runner
              defaults. Exits 0 when every required job succeeded, where
              `publish` is required on a tag run alone.
    warm    : Renders the Warm gate summary. Reads the `checks` and `wheels`
              results plus the GitHub-runner defaults. Exits 0 when both
              succeeded.

Each subcommand renders into `$GITHUB_STEP_SUMMARY` when it is set, stdout
otherwise.
"""

from jinja2  import Environment, FileSystemLoader
from json    import loads as json_loads
from os      import environ
from pathlib import Path
from sys     import argv
from tomllib import loads
from yaml    import safe_load


class Summary:
    """
    Renders a Prose workflow step summary.
    """

    def __init__(self):
        """
        Builds the summary from the run's environment, pointing a template
        loader at `.github/scripts/templates/` and declaring the ref, the
        tag flag, and the links the templates read as globals.
        """
        self.here = Path(".github/scripts")

        ref  = environ["REF"]
        repo = environ["GITHUB_REPOSITORY"]
        sha  = environ["SHA"]
        base = f"{environ['GITHUB_SERVER_URL']}/{repo}"

        self.is_tag = environ.get("GITHUB_REF_TYPE") == "tag"
        self.needs  = json_loads(environ.get("NEEDS", "{}"))
        self.env    = Environment(
            keep_trailing_newline = True,
            loader                = FileSystemLoader(self.here / "templates"),
            lstrip_blocks         = True,
            trim_blocks           = True
        )
        self.env.globals.update(
            codecov_url = f"https://app.codecov.io/gh/{repo}/commit/{sha}",
            commit_link = f"[`{sha[:7]}`]({base}/commit/{sha})",
            is_tag      = self.is_tag,
            pypi_url    = f"https://pypi.org/project/prose-formatter/{ref}/",
            ref         = ref,
            tag_link    = f"[`{ref}`]({base}/releases/tag/{ref})"
        )

    def _emit(self, name: str, **context):
        """
        Renders the `name` template into `$GITHUB_STEP_SUMMARY`, or to
        stdout when it is unset.
        """
        text = self.env.get_template(f"{name}-summary.md.j2").render(**context)

        if path := environ.get("GITHUB_STEP_SUMMARY"):
            with open(path, "a", encoding="utf-8") as f:
                f.write(text)
        else:
            print(text, end = "")

    def _failed(self, *jobs: str) -> bool:
        """
        Reports whether any of the `jobs` ended in anything other than
        success.
        """
        return any(self._result(job) != "success" for job in jobs)

    def _gate(self, name: str, *jobs: str, **context):
        """
        Renders the `name` template and exits with the verdict of the
        `jobs`.
        """
        verdict = self._failed(*jobs)

        self._emit(name, check_mark = "❌" if verdict else "✅", **context)
        raise SystemExit(verdict)

    def _output(self, job: str, name: str) -> str:
        """
        Returns the `job`'s `name` output, empty when the job declared none.
        """
        return self.needs[job].get("outputs", {}).get(name, "")

    def _result(self, job: str) -> str:
        """
        Returns the result the `job` finished with.
        """
        return self.needs[job]["result"]

    def _row(self, job: str, jobs: dict) -> dict:
        """
        Returns the mark and the name one corpus row shows.

        Args:
            job  : The job id whose row to build.
            jobs : The workflow's job table.
        """
        return {
            "mark" : {"skipped": "➖", "success": "✅"}.get(self._result(job), "❌"),
            "name" : jobs[job]["name"].split("${{")[0].strip() or job.capitalize()
        }

    def ci(self):
        """
        Renders the CI gate summary and exits with the matrix verdict.
        """
        self._gate("ci", "check")

    def corpus(self):
        """
        Renders the Corpus gate summary and exits with the verdict of every
        job the workflow's gate depends on, read off the workflow itself,
        the delta reporting without gating.
        """
        workflow = Path(".github/workflows/corpus.yml").read_text(encoding="utf-8")
        jobs     = safe_load(workflow)["jobs"]
        needed   = jobs["gate"]["needs"]

        self._gate(
            "corpus",
            *[
                job for job in needed
                if job != "delta" and self._result(job) != "skipped"
            ],
            sweeps = [self._row(job, jobs) for job in needed]
        )

    def deploy(self):
        """
        Renders the Deploy gate summary and exits with the deploy verdict.
        """
        stages = {
            "checks" : "Checks",
            "press"  : "Press",
            "deploy" : "Publish"
        }  # prose: skip

        self._gate(
            "deploy",
            *stages,
            stage = next((s for job, s in stages.items() if self._failed(job)), ""),
            url   = self._output("deploy", "url")
        )

    def draft(self):
        """
        Renders the Draft summary across the cut, existing, and no-op
        states.
        """
        self._emit(
            "draft",
            draft_url = environ.get("DRAFT_URL", ""),
            state     = environ.get("DRAFT_STATE", ""),
            version   = environ["VERSION"]
        )

    def release(self):
        """
        Renders the Release gate summary and exits with the pipeline
        verdict.
        """
        artifacts = [
            {
                "label"  : p["label"],
                "mark"   : "✅" if any(Path("dist").glob(p["pattern"])) else "❌",
                "target" : f"`{p['target']}`" if p.get("target") else "—"
            }
            for p in loads((self.here / "platforms.toml").read_text())["platforms"]
        ]

        prepub_failed = self._failed("build", "sdist", "validate")
        published     = not self._failed("publish")

        self._emit(
            "release",
            platforms     = artifacts,
            prepub_failed = prepub_failed,
            published     = published
        )

        raise SystemExit(prepub_failed or self.is_tag and not published)

    def warm(self):
        """
        Renders the Warm gate summary and exits with the cache-warm verdict.
        """
        self._gate("warm", "checks", "wheels")


if __name__ == "__main__":

    if (cmd := argv[1]) not in {n for n in vars(Summary) if not n.startswith("_")}:
        raise SystemExit(f"unknown subcommand: {cmd}")
    getattr(Summary(), cmd)()
