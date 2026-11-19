#!/usr/bin/env -S uv run --script
# MISE description="Rewrite README site/ paths to raw URLs at the release tag or main"
# /// script
# requires-python = ">=3.11"
# ///
"""
Rewrites the README's relative `site/` paths to absolute raw URLs at the tag
a release run publishes, or at `main` in a run publishing no tag.

Handles the Markdown link form `](../site/...)` and the HTML attribute forms
`src="../site/..."` and `srcset="../site/..."`.
"""

from os      import environ
from pathlib import Path
from re      import sub

RAW = "https://github.com/Jybbs/prose/raw/"


def absolute(readme: str, ref: str) -> str:
    """
    Rewrites every relative `site/` path in a Markdown link, a `src`, or a
    `srcset` of `readme` under the raw URL of `ref`.

    Returns:
        The rewritten `readme`.
    """
    return sub(r'(\]\(|src="|srcset=")\.\./(?=site/)', rf"\g<1>{RAW}{ref}/", readme)


def main():
    """
    Rewrites `.github/README.md` in place under the raw URL of the run's
    ref.
    """
    readme = Path(".github/README.md")
    readme.write_text(
        absolute(readme.read_text(encoding="utf-8"), ref()),
        encoding = "utf-8"
    )


def ref() -> str:
    """
    Reads the ref the image addresses pin to from the run's environment.

    Returns:
        The tag a tag-triggered run publishes, or `main` in any other run.
    """
    return (
        environ["GITHUB_REF_NAME"]
        if environ.get("GITHUB_REF_TYPE") == "tag"
        else "main"
    )


if __name__ == "__main__":

    main()
