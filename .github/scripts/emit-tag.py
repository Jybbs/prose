#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Emit the release tag for `draft.yml` to consume.

Reads `[package].version` from `Cargo.toml` and writes `version=<tag>`
to `$GITHUB_OUTPUT`.
"""

from os      import environ
from pathlib import Path
from tomllib import loads


if __name__ == "__main__":

    tag = loads(Path("Cargo.toml").read_text())["package"]["version"]

    with open(environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as f:
        f.write(f"version={tag}\n")
