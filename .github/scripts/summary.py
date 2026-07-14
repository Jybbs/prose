#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["jinja2==3.1.6"]
# ///
"""
Render a Prose step summary and gate the workflow's exit code.

Subcommands:
    ci       Render the CI gate summary. Reads `CHECK` plus the
             GitHub-runner defaults. Exits 0 when `CHECK` is success.
    deploy   Render the Deploy gate summary. Reads `DEPLOY` and `URL`
             plus the GitHub-runner defaults. Exits 0 when `DEPLOY` is
             success.
    draft    Render the Draft summary. Reads `DRAFT_STATE`,
             `DRAFT_URL`, and `VERSION`, plus the GitHub-runner
             defaults.
    release  Render the Release gate summary. Reads `BUILD`, `SDIST`,
             `VALIDATE`, `PUBLISH`, plus the GitHub-runner defaults.
             Exits 0 when every required job succeeded. `PUBLISH` is
             required only on tag runs.
    warm     Render the Warm gate summary. Reads `CHECKS` and `WHEELS`
             plus the GitHub-runner defaults. Exits 0 when both are
             success.

Each subcommand appends to `$GITHUB_STEP_SUMMARY`.
"""

from jinja2  import Environment, FileSystemLoader
from os      import environ
from pathlib import Path
from sys     import argv
from tomllib import loads


def failed(*signals: str) -> bool:
    """
    Report whether any of the `signals` env vars is not a success.
    """
    return any(environ[signal] != "success" for signal in signals)


class Summary:
    """
    Render a Prose workflow step summary.
    """

    def __init__(self):
        self.here = Path(__file__).parent

        ref  = environ["REF"]
        repo = environ["GITHUB_REPOSITORY"]
        sha  = environ["SHA"]
        base = f"{environ['GITHUB_SERVER_URL']}/{repo}"

        self.is_tag = environ.get("GITHUB_REF_TYPE") == "tag"
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
        Render the `name` template with `context` and append to `$GITHUB_STEP_SUMMARY`.
        """
        template = self.env.get_template(f"{name}-summary.md.j2")

        with open(environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as f:
            f.write(template.render(**context))

    def _gate(self, name: str, *signals: str, **context):
        """
        Render the `name` template and exit with the verdict of the `signals` env vars.
        """
        verdict = failed(*signals)
        self._emit(name, check_mark = "❌" if verdict else "✅", **context)
        raise SystemExit(verdict)

    def ci(self):
        """
        Render the CI gate summary and exit with the matrix verdict.
        """
        self._gate("ci", "CHECK")

    def deploy(self):
        """
        Render the Deploy gate summary and exit with the deploy verdict.
        """
        self._gate("deploy", "DEPLOY", url = environ.get("URL", ""))

    def draft(self):
        """
        Render the Draft summary across the cut, existing, and no-op states.
        """
        self._emit(
            "draft",
            draft_url = environ.get("DRAFT_URL", ""),
            state     = environ.get("DRAFT_STATE", ""),
            version   = environ["VERSION"]
        )

    def release(self):
        """
        Render the Release gate summary and exit with the pipeline verdict.
        """
        platforms = loads((self.here / "platforms.toml").read_text())["platforms"]
        artifacts = [
            {
                "label"  : p["label"],
                "mark"   : "✅" if path else "❌",
                "target" : f"`{p['target']}`" if p.get("target") else "—"
            }
            for p in platforms
            for path in [next(Path("dist").glob(p["pattern"]), None)]
        ]

        prepub_failed = failed("BUILD", "SDIST", "VALIDATE")
        published     = not failed("PUBLISH")

        self._emit(
            "release",
            platforms     = artifacts,
            prepub_failed = prepub_failed,
            published     = published
        )

        raise SystemExit(prepub_failed or (self.is_tag and not published))

    def warm(self):
        """
        Render the Warm gate summary and exit with the cache-warm verdict.
        """
        self._gate("warm", "CHECKS", "WHEELS")


if __name__ == "__main__":

    if (cmd := argv[1]) not in {n for n in vars(Summary) if not n.startswith("_")}:
        raise SystemExit(f"unknown subcommand: {cmd}")
    getattr(Summary(), cmd)()
