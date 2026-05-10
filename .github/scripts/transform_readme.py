#!/usr/bin/env python3
"""
Rewrite the README's relative `assets/` paths to absolute raw URLs.

Handles both Markdown link form `](assets/...)` and HTML attribute
form `src="assets/..."`.
"""

from pathlib import Path
from re      import sub


if __name__ == "__main__":

    prefix = "https://github.com/Jybbs/prose/raw/main/"
    readme = Path("README.md")

    content = readme.read_text(encoding="utf-8")
    content = sub(r"\]\((assets/[^)]+)\)", rf"]({prefix}\g<1>)",   content)
    content = sub(r'src="(assets/[^"]+)"', rf'src="{prefix}\g<1>"', content)
    readme.write_text(content, encoding="utf-8")
