#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["jinja2"]
# ///
"""
Render a Prose step summary and gate the workflow's exit code.

Subcommands:
    ci       Render the CI gate summary. Reads `CHECK` plus the
             GitHub-runner defaults. Exits 0 when `CHECK` is success.
    draft    Render the Draft summary. Reads `DRAFT_URL` and
             `VERSION`, plus the GitHub-runner defaults.
    release  Render the Release gate summary. Reads `BUILD`, `SDIST`,
             `VALIDATE`, `PUBLISH`, plus the GitHub-runner defaults.
             Exits 0 when every required job succeeded. `PUBLISH` is
             required only on tag runs.

Each subcommand appends to `$GITHUB_STEP_SUMMARY`.
"""

from jinja2  import Environment, FileSystemLoader
from os      import environ
from pathlib import Path
from sys     import argv
from tomllib import loads


class Summary:
    """
    Render a Prose CI, Draft, or Release step summary.
    """

    def __init__(self):
        here = Path(__file__).parent
        ref  = environ["REF"]
        repo = environ["GITHUB_REPOSITORY"]
        sha  = environ["SHA"]
        base = f"{environ['GITHUB_SERVER_URL']}/{repo}"

        self.is_tag = environ.get("GITHUB_REF_TYPE") == "tag"
        self.env    = Environment(
            keep_trailing_newline = True,
            loader                = FileSystemLoader(here / "templates"),
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
        self.platforms = loads((here / "platforms.toml").read_text())["platforms"]

    def _emit(self, template: str, **context):
        """
        Render `template` with `context` and append to `$GITHUB_STEP_SUMMARY`.
        """
        with open(environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as f:
            f.write(self.env.get_template(template).render(**context))

    def ci(self):
        """
        Render the CI gate summary and exit with the matrix verdict.
        """
        failed = environ["CHECK"] != "success"
        self._emit(
            "ci-summary.md.j2",
            check_mark = "❌" if failed else "✅"
        )
        raise SystemExit(failed)

    def draft(self):
        """
        Render the Draft summary with the cut release-draft URL.
        """
        self._emit(
            "draft-summary.md.j2",
            draft_url = environ["DRAFT_URL"],
            version   = environ["VERSION"]
        )

    def release(self):
        """
        Render the Release gate summary and exit with the pipeline verdict.
        """
        artifacts = [
            {
                "label":  p["label"],
                "mark":   "✅" if path else "❌",
                "target": f"`{p['target']}`" if p.get("target") else "—"
            }
            for p in self.platforms
            for path in [next(Path("dist").glob(p["pattern"]), None)]
        ]
        status = {k.lower(): environ[k] for k in [
            "BUILD", "PUBLISH", "SDIST", "VALIDATE"
        ]}
        prepub_failed = any(
            status[k] != "success" for k in ["build", "sdist", "validate"]
        )

        self._emit(
            "release-summary.md.j2",
            **status,
            platforms     = artifacts,
            prepub_failed = prepub_failed
        )

        raise SystemExit(
            prepub_failed or (self.is_tag and status["publish"] != "success")
        )


if __name__ == "__main__":

    if (cmd := argv[1]) not in {"ci", "draft", "release"}:
        raise SystemExit(f"unknown subcommand: {cmd}")
    getattr(Summary(), cmd)()
