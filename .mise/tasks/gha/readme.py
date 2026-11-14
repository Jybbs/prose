#!/usr/bin/env -S uv run --script
# MISE description="Rewrite the README's site/ paths to absolute raw URLs for PyPI"
# /// script
# requires-python = ">=3.11"
# ///
"""
Rewrites the README's relative `site/` paths to absolute raw URLs.

Handles the Markdown link form `](../site/...)` and the HTML attribute forms
`src="../site/..."` and `srcset="../site/..."`.
"""

from pathlib import Path
from re      import sub

PREFIX = "https://github.com/Jybbs/prose/raw/main/"


def absolute(readme: str) -> str:
    """
    Returns `readme` with every relative `site/` path in a Markdown link, a
    `src`, or a `srcset` rewritten under `PREFIX`.
    """
    return sub(r'(\]\(|src="|srcset=")\.\./(?=site/)', rf"\g<1>{PREFIX}", readme)


if __name__ == "__main__":

    readme = Path(".github/README.md")
    readme.write_text(absolute(readme.read_text(encoding="utf-8")), encoding="utf-8")
