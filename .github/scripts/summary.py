#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["jinja2"]
# ///
"""
Render a 🪻 Prose step summary and gate the workflow's exit code.

Subcommands:
    ci       Render the CI gate summary (reads `RESULT` plus the GitHub
             defaults). Exits 0 on `success`, 1 otherwise.
    release  Render the Release gate summary (reads `BUILD`, `SDIST`,
             `PUBLISH` plus the GitHub defaults). Exits 0 when every
             required job succeeded, 1 otherwise.

Both subcommands append to `$GITHUB_STEP_SUMMARY`.
"""

from jinja2  import Environment, FileSystemLoader
from os      import environ
from pathlib import Path
from sys     import argv

ENV = Environment(
    keep_trailing_newline = True,
    loader                = FileSystemLoader(Path(__file__).with_name("templates")),
    lstrip_blocks         = True,
    trim_blocks           = True
)
PLATFORMS = [
    ("🐧 Linux x86_64",   "*manylinux*x86_64.whl",  "x86_64-unknown-linux-gnu"),
    ("🐧 Linux aarch64",  "*manylinux*aarch64.whl", "aarch64-unknown-linux-gnu"),
    ("🍎 macOS x86_64",   "*macosx*x86_64.whl",     "x86_64-apple-darwin"),
    ("🍎 macOS aarch64",  "*macosx*arm64.whl",      "aarch64-apple-darwin"),
    ("🪟 Windows x86_64", "*win_amd64.whl",         "x86_64-pc-windows-msvc"),
    ("🚢 sdist",          "*.tar.gz",               None)
]
REPO_URL  = f"{environ['GITHUB_SERVER_URL']}/{environ['GITHUB_REPOSITORY']}"
SHA       = environ["GITHUB_SHA"]


def ci():
    """
    Render the CI gate summary and exit with the matrix verdict.

    Reads `RESULT` plus the GitHub-runner defaults, renders
    `ci-summary.md.j2`, and exits 0 when `RESULT == "success"`,
    1 otherwise.
    """
    REF    = environ.get("GITHUB_HEAD_REF") or environ["GITHUB_REF_NAME"]
    RESULT = environ["RESULT"]
    emit(
        "ci-summary.md.j2",
        commit_url = f"{REPO_URL}/commit/{SHA}",
        msg        = f"Result `{RESULT}`",
        ref        = REF,
        short      = SHA[:7],
        tree_url   = f"{REPO_URL}/tree/{REF}"
    )
    raise SystemExit(RESULT != "success")


def emit(template: str, **vars):
    """
    Render `template` with `vars` and append to `$GITHUB_STEP_SUMMARY`.

    Args:
        template : Filename of a `.md.j2` template under `templates/`.
        vars     : Substitution context passed to `Template.render`.
    """
    with open(environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as f:
        f.write(ENV.get_template(template).render(**vars))


def release():
    """
    Render the Release gate summary and exit with the pipeline verdict.

    Reads `BUILD`, `SDIST`, `PUBLISH` plus the GitHub-runner defaults,
    renders `release-summary.md.j2`, and exits 0 when every required
    job (build, sdist, plus publish on tag runs) succeeded.
    """
    BUILD    = environ["BUILD"]
    IS_TAG   = environ.get("GITHUB_REF_TYPE") == "tag"
    PUBLISH  = environ["PUBLISH"]
    REF_NAME = environ["GITHUB_REF_NAME"]
    SDIST    = environ["SDIST"]
    VERSION   = REF_NAME.removeprefix("v")
    platforms = [
        (label, target, next(Path("dist").glob(pattern), None))
        for label, pattern, target in PLATFORMS
    ]
    emit(
        "release-summary.md.j2",
        build       = BUILD,
        commit_link = f"[`{SHA[:7]}`]({REPO_URL}/commit/{SHA})",
        is_tag      = IS_TAG,
        platforms   = platforms,
        publish     = PUBLISH,
        pypi_url    = f"https://pypi.org/project/prose-formatter/{VERSION}/",
        sdist       = SDIST,
        tag_link    = f"[`{REF_NAME}`]({REPO_URL}/releases/tag/{REF_NAME})",
        tree_link   = f"[`{REF_NAME}`]({REPO_URL}/tree/{REF_NAME})",
        version     = VERSION
    )
    raise SystemExit(
        BUILD != "success"
        or SDIST != "success"
        or (IS_TAG and PUBLISH != "success")
    )


{"ci": ci, "release": release}[argv[1]]()
